use std::convert::TryFrom;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::str;

mod commands;
mod error;

pub use commands::*;
pub use error::*;

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
    child: Child,
    stdout: Option<BufReader<ChildStdout>>,
}

impl SvnlookCommand {
    fn spawn(cmd: &mut Command) -> Result<Self, SvnError> {
        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;

        let stdout = child.stdout.take().unwrap();
        Ok(Self {
            child,
            stdout: Some(BufReader::new(stdout)),
        })
    }

    fn handle_io<F: FnOnce(&mut BufReader<ChildStdout>) -> io::Result<usize>>(
        &mut self,
        handler: F,
    ) -> io::Result<usize> {
        let res = self
            .stdout
            .as_mut()
            .map(handler)
            .unwrap_or(Err(io::Error::new(io::ErrorKind::Other, "closed")));

        if let Ok(0) = res {
            match self.finish() {
                Ok(status) if !status.success() => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "svnlook exited nonzero",
                    ))
                }
                Err(SvnError::CommandError(e)) => return Err(e),
                _ => (),
            }
        }

        res
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
        self.handle_io(|r| r.read(buf))
    }

    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut]) -> io::Result<usize> {
        self.handle_io(|r| r.read_vectored(bufs))
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
