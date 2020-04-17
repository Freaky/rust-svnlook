use std::convert::TryFrom;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::str;

use chrono::{DateTime, FixedOffset};

mod display;
mod parse;
mod error;

pub use error::*;
pub use display::*;

#[derive(Debug, Clone)]
pub struct Svnlook {
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SvnInfo {
    pub revision: u64,
    pub committer: String,
    pub date: DateTime<FixedOffset>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SvnStatus {
    Added,
    Copied(SvnFrom),
    Deleted,
    Updated,
    PropChange,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct SvnFrom {
    pub path: PathBuf,
    pub revision: u64,
}

#[derive(Debug, Clone)]
pub struct SvnChange {
    pub path: PathBuf,
    pub status: SvnStatus,
}

#[derive(Debug)]
pub struct SvnlookCommand {
    child: Child,
    stdout: Option<BufReader<ChildStdout>>,
}

impl SvnlookCommand {
    fn spawn(mut cmd: Command) -> Result<Self, SvnError> {
        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;

        let stdout = child.stdout.take().unwrap();
        Ok(Self {
            child,
            stdout: Some(BufReader::new(stdout)),
        })
    }

    pub fn finish(&mut self) -> Result<ExitStatus, SvnError> {
        self.stdout = None;

        Ok(self.child.wait()?)
    }
}

impl Drop for SvnlookCommand {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

impl Read for SvnlookCommand {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stdout
            .as_mut()
            .map(|s| s.read(buf))
            .unwrap_or(Err(io::Error::new(io::ErrorKind::Other, "closed")))
    }
}

impl BufRead for SvnlookCommand {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.stdout
            .as_mut()
            .map(|s| s.fill_buf())
            .unwrap_or(Err(io::Error::new(io::ErrorKind::Other, "closed")))
    }

    fn consume(&mut self, amt: usize) {
        self.stdout.as_mut().map(|s| s.consume(amt));
    }
}

impl<P: AsRef<Path>> From<P> for Svnlook {
    fn from(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl Svnlook {
    pub fn new<R: AsRef<Path>>(path: R) -> Self {
        Self::from(path)
    }

    pub fn youngest(&self) -> Result<u64, SvnError> {
        let n = Command::new("svnlook")
            .arg("youngest")
            .arg("--")
            .arg(&self.path)
            .output()?;

        if !n.status.success() {
            return Err(SvnError::ExitFailure(n.status));
        }

        str::from_utf8(&n.stdout[..])?
            .trim()
            .parse()
            .map_err(SvnError::from)
    }

    pub fn info(&self, revision: u64) -> Result<SvnInfo, SvnError> {
        let n = Command::new("svnlook")
            .arg("info")
            .arg("-r")
            .arg(revision.to_string())
            .arg("--")
            .arg(&self.path)
            .output()?;

        if !n.status.success() {
            return Err(SvnError::ExitFailure(n.status));
        }

        SvnInfo::try_from((revision, &n.stdout[..]))
    }

    pub fn changed(&self, revision: u64) -> Result<SvnChangedIter, SvnError> {
        let mut cmd = Command::new("svnlook");
        cmd.args(&["changed", "--copy-info", "-r"])
            .arg(revision.to_string())
            .arg("--")
            .arg(&self.path);

        Ok(SvnChangedIter::from(SvnlookCommand::spawn(cmd)?))
    }

    pub fn diff(&self, revision: u64) -> Result<SvnlookCommand, SvnError> {
        let mut cmd = Command::new("svnlook");
        cmd.arg("--ignore-properties")
            .arg("--diff-copy-from")
            .arg("diff")
            .arg("-r")
            .arg(revision.to_string())
            .arg("--")
            .arg(&self.path);

        SvnlookCommand::spawn(cmd)
    }

    pub fn cat<R: AsRef<Path>>(
        &self,
        revision: u64,
        filename: R,
    ) -> Result<SvnlookCommand, SvnError> {
        let mut cmd = Command::new("svnlook");
        cmd.arg("cat")
            .arg("-r")
            .arg(revision.to_string())
            .arg("--")
            .arg(&self.path)
            .arg(filename.as_ref().as_os_str());

        SvnlookCommand::spawn(cmd)
    }
}

pub struct SvnChangedIter {
    svnlook: SvnlookCommand,
    line: Vec<u8>,
    finished: bool,
}

impl From<SvnlookCommand> for SvnChangedIter {
    fn from(cmd: SvnlookCommand) -> Self {
        Self {
            svnlook: cmd,
            line: vec![],
            finished: false,
        }
    }
}

impl Drop for SvnChangedIter {
    fn drop(&mut self) {
        let _ = self.svnlook.finish();
    }
}

impl SvnChangedIter {
    fn parse(&mut self) -> Result<SvnChange, SvnError> {
        let mut change = SvnChange::try_from(&self.line[..])?;
        self.line.clear();

        if let SvnStatus::Copied(_) = change.status {
            self.svnlook.read_until(b'\n', &mut self.line)?;
            change.status = SvnStatus::Copied(SvnFrom::try_from(&self.line[..])?);
        }

        Ok(change)
    }
}

impl Iterator for SvnChangedIter {
    type Item = Result<SvnChange, SvnError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.line.clear();

        if self.finished {
            return None;
        }

        match self.svnlook.read_until(b'\n', &mut self.line) {
            Ok(0) => {
                self.finished = true;
                match self.svnlook.finish() {
                    Ok(status) if status.success() => None,
                    Ok(status) => Some(Err(SvnError::ExitFailure(status))),
                    Err(e) => Some(Err(e)),
                }
            }
            Ok(_) => Some(self.parse()),
            Err(e) => Some(Err(SvnError::from(e))),
        }
    }
}
