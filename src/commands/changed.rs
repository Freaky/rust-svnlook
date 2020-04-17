use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::io::BufRead;
use std::path::PathBuf;
use std::str::FromStr;

use super::try_chomp;
use crate::{SvnError, SvnlookCommand};

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

impl TryFrom<&[u8]> for SvnChange {
    type Error = SvnError;

    fn try_from(line: &[u8]) -> Result<Self, Self::Error> {
        let line = try_chomp(line)?;

        if line.len() < 4 {
            return Err(SvnError::ParseError);
        }

        let (change, path) = line.split_at(4);
        Ok(SvnChange {
            path: PathBuf::from(String::from_utf8_lossy(path).to_string()),
            status: change.try_into()?,
        })
    }
}

impl TryFrom<&[u8]> for SvnFrom {
    type Error = SvnError;

    fn try_from(line: &[u8]) -> Result<Self, Self::Error> {
        let line = try_chomp(line)?;

        if !line.starts_with(b"    (from ") || !line.ends_with(b")") {
            return Err(SvnError::ParseError);
        }

        let line: &[u8] = &line[10..line.len() - 1];
        line.iter()
            .rposition(|&b| b == b':')
            .map(|pos| line.split_at(pos))
            .filter(|(_, revision)| revision.len() > 2)
            .ok_or(SvnError::ParseError)
            .and_then(|(path, revision)| {
                std::str::from_utf8(&revision[2..])
                    .map_err(SvnError::from)
                    .and_then(|s| u64::from_str(s).map_err(SvnError::from))
                    .map(|revision| SvnFrom {
                        path: PathBuf::from(String::from_utf8_lossy(path).to_string()),
                        revision,
                    })
            })
    }
}

#[derive(Debug)]
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
