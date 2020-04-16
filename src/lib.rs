use chrono::prelude::*;
use chrono::DateTime;

use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::str::{self, FromStr};

#[derive(Debug)]
pub enum SvnError {
    CommandError(io::Error),
    ExitFailure(std::process::ExitStatus),
    ParseError,
}

impl Error for SvnError {}

impl fmt::Display for SvnError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SvnError::CommandError(io) => io.fmt(f),
            SvnError::ExitFailure(status) => write!(f, "non-zero exit from command: {}", status),
            SvnError::ParseError => write!(f, "parse error"),
        }
    }
}

impl From<io::Error> for SvnError {
    fn from(err: io::Error) -> Self {
        SvnError::CommandError(err)
    }
}

impl From<std::str::Utf8Error> for SvnError {
    fn from(_err: std::str::Utf8Error) -> Self {
        SvnError::ParseError
    }
}

impl From<std::num::ParseIntError> for SvnError {
    fn from(_err: std::num::ParseIntError) -> Self {
        SvnError::ParseError
    }
}

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

impl TryFrom<&[u8]> for SvnStatus {
    type Error = SvnError;

    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        if s.len() < 3 {
            return Err(SvnError::ParseError);
        }

        Ok(match &s[0..3] {
            b"A  " => SvnStatus::Added,
            b"A +" => SvnStatus::Copied(SvnFrom::default()),
            b"D  " => SvnStatus::Deleted,
            b"U  " => SvnStatus::Updated,
            b"_U " => SvnStatus::PropChange,
            b"UU " => SvnStatus::Updated,
            _ => return Err(SvnError::ParseError),
        })
    }
}

impl fmt::Display for SvnStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SvnStatus::Added => "Added",
                SvnStatus::Copied(_) => "Copied",
                SvnStatus::Deleted => "Deleted",
                SvnStatus::Updated => "Updated",
                SvnStatus::PropChange => "PropChange",
            }
        )
    }
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

        let mut lines = n.stdout.splitn(4, |b| *b == b'\n');

        let committer = lines
            .next()
            .map(String::from_utf8_lossy)
            .ok_or(SvnError::ParseError)?;

        let date = lines
            .next()
            .filter(|d| d.len() > 25)
            .and_then(|d| str::from_utf8(&d[0..25]).ok())
            .and_then(|d| DateTime::parse_from_str(d, "%Y-%m-%d %H:%M:%S %z").ok())
            .ok_or(SvnError::ParseError)?;

        let bytes = lines
            .next()
            .and_then(|d| str::from_utf8(d).ok())
            .and_then(|d| usize::from_str(d).ok())
            .ok_or(SvnError::ParseError)?;

        lines
            .next()
            .filter(|m| m.len() > bytes)
            .map(|m| &m[0..bytes])
            .map(String::from_utf8_lossy)
            .map(|msg| SvnInfo {
                revision,
                committer: committer.to_string(),
                date,
                message: msg.to_string(),
            })
            .ok_or(SvnError::ParseError)
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

fn chomp(slice: &[u8]) -> &[u8] {
    if slice.ends_with(b"\n") {
        &slice[..slice.len() - 1]
    } else {
        slice
    }
}

impl SvnChangedIter {
    fn parse(&mut self) -> Result<SvnChange, SvnError> {
        if self.line.len() < 4 {
            return Err(SvnError::ParseError);
        }

        let (change, path) = self.line.split_at(4);
        let mut change = SvnChange {
            path: PathBuf::from(String::from_utf8_lossy(chomp(path)).to_string()),
            status: change.try_into()?,
        };
        self.line.clear();

        if let SvnStatus::Copied(_) = change.status {
            self.svnlook.read_until(b'\n', &mut self.line)?;
            let line = chomp(&self.line);

            if !line.starts_with(b"    (from ") || !line.ends_with(b")") {
                return Err(SvnError::ParseError);
            }

            let line: &[u8] = &line[10..line.len() - 1];
            let from = line
                .iter()
                .rposition(|&b| b == b':')
                .map(|pos| line.split_at(pos))
                .filter(|(_, revision)| revision.len() > 2)
                .ok_or(SvnError::ParseError)
                .and_then(|(path, revision)| {
                    str::from_utf8(&revision[2..])
                        .map_err(SvnError::from)
                        .and_then(|s| u64::from_str(s).map_err(SvnError::from))
                        .map(|revision| SvnFrom {
                            path: PathBuf::from(String::from_utf8_lossy(path).to_string()),
                            revision,
                        })
                })?;
            change.status = SvnStatus::Copied(from);
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
