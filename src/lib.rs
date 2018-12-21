use chrono::prelude::*;
use chrono::DateTime;

use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::str;
use std::str::FromStr;

#[derive(Debug)]
pub enum SvnError {
    CommandError(io::Error),
    ExitFailure(std::process::ExitStatus),
    ParseError,
}

impl fmt::Display for SvnError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for SvnError {
    fn description(&self) -> &str {
        match self {
            SvnError::CommandError(io) => io.description(),
            SvnError::ExitFailure(status) => "non-zero exit from command",
            SvnError::ParseError => "parse error",
        }
    }
}

impl From<io::Error> for SvnError {
    fn from(err: io::Error) -> Self {
        SvnError::CommandError(err)
    }
}

impl From<std::str::Utf8Error> for SvnError {
    fn from(err: std::str::Utf8Error) -> Self {
        SvnError::ParseError
    }
}

impl From<std::num::ParseIntError> for SvnError {
    fn from(err: std::num::ParseIntError) -> Self {
        SvnError::ParseError
    }
}

#[derive(Debug)]
pub struct SvnRepo {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct SvnInfo {
    pub revision: u32,
    pub committer: String,
    pub date: DateTime<FixedOffset>,
    pub message: String,
}

#[derive(Debug)]
pub enum SvnStatus {
    Added,
    Copied,
    Deleted,
    Updated,
    PropChange,
}

impl SvnStatus {
    fn from_bytes(s: &[u8]) -> Result<Self, SvnError> {
        if s.len() < 3 {
            return Err(SvnError::ParseError);
        }

        Ok(match &s[0..3] {
            b"A  " => SvnStatus::Added,
            b"A +" => SvnStatus::Copied,
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
                SvnStatus::Copied => "Copied",
                SvnStatus::Deleted => "Deleted",
                SvnStatus::Updated => "Updated",
                SvnStatus::PropChange => "PropChange",
            }
        )
    }
}

#[derive(Debug)]
pub struct Delta {
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug)]
pub struct SvnFrom {
    pub path: PathBuf,
    pub revision: u32,
}

#[derive(Debug)]
pub struct SvnChange {
    pub path: PathBuf,
    pub status: SvnStatus,
    pub from: Option<SvnFrom>,
    pub delta: Option<Delta>,
}

impl SvnRepo {
    pub fn new<R: AsRef<Path>>(path: R) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn youngest(&self) -> Result<u32, SvnError> {
        let n = Command::new("svnlook")
            .arg("youngest")
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

    pub fn info(&self, revision: u32) -> Result<SvnInfo, SvnError> {
        let n = Command::new("svnlook")
            .arg("info")
            .arg("-r")
            .arg(revision.to_string())
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

    // iterator?
    pub fn changed(&self, revision: u32) -> Result<Vec<SvnChange>, SvnError> {
        let n = Command::new("svnlook")
            .arg("--copy-info")
            .arg("changed")
            .arg("-r")
            .arg(revision.to_string())
            .arg(&self.path)
            .output()?;

        if !n.status.success() {
            return Err(SvnError::ExitFailure(n.status));
        }

        let mut changes = vec![];
        let mut lines = n.stdout.split(|&b| b == b'\n').filter(|s| s.len() > 4);

        while let Some(line) = lines.next() {
            let (change, path) = line.split_at(4);
            let mut change = SvnChange {
                path: PathBuf::from(OsStr::from_bytes(path)),
                status: SvnStatus::from_bytes(change)?,
                from: None,
                delta: None,
            };

            if let SvnStatus::Copied = change.status {
                change.from = lines
                    .next()
                    .filter(|line| line.starts_with(b"    (from ") && line.ends_with(b")"))
                    .map(|line| &line[10..line.len() - 1])
                    .and_then(|line| {
                        line.iter()
                            .rposition(|&b| b == b':')
                            .map(|pos| line.split_at(pos))
                    })
                    .filter(|(_path, revision)| revision.len() > 2)
                    .map(|(path, revision)| SvnFrom {
                        path: PathBuf::from(OsStr::from_bytes(path)),
                        revision: str::from_utf8(&revision[2..])
                            .ok()
                            .and_then(|s| u32::from_str(s).ok())
                            .unwrap_or(0),
                    });
            }

            changes.push(change);
        }

        Ok(changes)
    }

    // io::Read?
    //
    // {Added,Modified,Deleted}: <filename>
    // ===================================================================
    // --- old_filename (rev \d+)
    // +++ new_filename yyyy-mm-dd hh:mm:ss UTC (rev \d+)
    //  <diff>
    //
    // {Added,Modified,Deleted}: <next_filename>
    pub fn diff<R: AsRef<Path>>(
        &self,
        revision: u32,
        filename: Option<R>,
    ) -> Result<Vec<u8>, SvnError> {
        let n = Command::new("svnlook")
            .arg("--ignore-properties")
            .arg("--diff-copy-from")
            .arg("diff")
            .arg("-r")
            .arg(revision.to_string())
            .arg(&self.path)
            .output()?;

        if !n.status.success() {
            return Err(SvnError::ExitFailure(n.status));
        }

        Ok(n.stdout)
    }

    // io::Read?
    pub fn cat<R: AsRef<Path>>(
        &self,
        revision: u32,
        filename: R,
        limit: Option<usize>,
    ) -> Result<Vec<u8>, SvnError> {
        let n = Command::new("svnlook")
            .arg("cat")
            .arg("-r")
            .arg(revision.to_string())
            .arg(&self.path)
            .arg(filename.as_ref().as_os_str())
            .output()?;

        if !n.status.success() {
            return Err(SvnError::ExitFailure(n.status));
        }

        Ok(n.stdout)
    }

    // iterator?
    pub fn diffstat(&self, revision: u32) {}
}
