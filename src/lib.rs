use std::convert::TryFrom;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::str;

mod commands;
mod child_reader;
mod error;

pub use commands::*;
pub use error::*;

use child_reader::ChildReader;

/// A struct representing the path to an svnlook binary
#[derive(Default, Debug, Clone)]
pub struct Svnlook {
    pub path: Option<PathBuf>,
}

/// An interface to an SVN repository using a given svnlook command
#[derive(Debug, Clone)]
pub struct Repository {
    svnlook: Svnlook,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct SvnlookCommand {
    child: BufReader<ChildReader>,
}

impl SvnlookCommand {
    fn spawn(cmd: &mut Command) -> Result<Self, SvnError> {
        let child = cmd.stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()?;

        Ok(Self {
            child: BufReader::new(ChildReader::from(child)),
        })
    }

    pub fn finish(&mut self) -> Result<ExitStatus, SvnError> {
        Ok(self.child.get_mut().finish()?)
    }
}

impl Read for SvnlookCommand {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.child.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut]) -> io::Result<usize> {
        self.child.read_vectored(bufs)
    }
}

impl BufRead for SvnlookCommand {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.child.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.child.consume(amt);
    }
}

impl<P: Into<PathBuf>> From<P> for Svnlook {
    fn from(path: P) -> Self {
        Self {
            path: Some(path.into()),
        }
    }
}

impl Svnlook {
    fn command(&self) -> Command {
        Command::new(
            self.path
                .as_ref()
                .map(|path| path.as_path())
                .unwrap_or(Path::new("svnlook")),
        )
    }

    pub fn repository<P: Into<PathBuf>>(&self, path: P) -> Repository {
        Repository::new_with_svnlook(path, self.clone())
    }
}

impl<P: Into<PathBuf>> From<P> for Repository {
    fn from(path: P) -> Self {
        Self {
            svnlook: Svnlook::default(),
            path: path.into(),
        }
    }
}

impl Repository {
    pub fn new<R: Into<PathBuf>>(path: R) -> Self {
        Self::from(path)
    }

    pub fn new_with_svnlook<R: Into<PathBuf>>(path: R, svnlook: Svnlook) -> Self {
        Self {
            svnlook,
            path: path.into(),
        }
    }

    pub fn youngest(&self) -> Result<u64, SvnError> {
        let n = self
            .svnlook
            .command()
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
        let n = self
            .svnlook
            .command()
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
        let mut cmd = self.svnlook.command();
        cmd.args(&["changed", "--copy-info", "-r"])
            .arg(revision.to_string())
            .arg("--")
            .arg(&self.path);

        Ok(SvnChangedIter::from(SvnlookCommand::spawn(&mut cmd)?))
    }

    pub fn diff(&self) -> SvnDiffBuilder {
        let mut cmd = self.svnlook.command();
        cmd.arg("diff").arg(&self.path);

        SvnDiffBuilder::from(cmd)
    }

    pub fn cat<R: AsRef<Path>>(
        &self,
        revision: u64,
        filename: R,
    ) -> Result<SvnlookCommand, SvnError> {
        let mut cmd = self.svnlook.command();
        cmd.arg("cat")
            .arg("-r")
            .arg(revision.to_string())
            .arg("--")
            .arg(&self.path)
            .arg(filename.as_ref().as_os_str());

        SvnlookCommand::spawn(&mut cmd)
    }
}
