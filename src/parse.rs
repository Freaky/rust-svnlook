use std::convert::{TryFrom, TryInto};
use std::path::PathBuf;
use std::str::{self, FromStr};

use chrono::DateTime;

use crate::{SvnChange, SvnError, SvnFrom, SvnInfo, SvnStatus};

fn try_chomp(slice: &[u8]) -> Result<&[u8], SvnError> {
    if slice.ends_with(b"\n") {
        Ok(&slice[..slice.len() - 1])
    } else {
        Err(SvnError::ParseError)
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

impl TryFrom<(u64, &[u8])> for SvnInfo {
    type Error = SvnError;

    fn try_from(info: (u64, &[u8])) -> Result<Self, Self::Error> {
        let (revision, bytes) = info;
        let mut lines = bytes.splitn(4, |b| *b == b'\n');

        let committer = lines
            .next()
            .map(String::from_utf8_lossy)
            .ok_or(SvnError::ParseError)?
            .to_string();

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

        let message = lines
            .next()
            .filter(|m| m.len() > bytes)
            .map(|m| &m[0..bytes])
            .map(String::from_utf8_lossy)
            .ok_or(SvnError::ParseError)?
            .to_string();

        Ok(SvnInfo {
            revision,
            committer,
            date,
            message,
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
                str::from_utf8(&revision[2..])
                    .map_err(SvnError::from)
                    .and_then(|s| u64::from_str(s).map_err(SvnError::from))
                    .map(|revision| SvnFrom {
                        path: PathBuf::from(String::from_utf8_lossy(path).to_string()),
                        revision,
                    })
            })
    }
}
