
use std::convert::TryFrom;
use std::str::{self, FromStr};

use chrono::{DateTime, FixedOffset};

use crate::SvnError;

#[derive(Debug, Clone, PartialEq)]
pub struct SvnInfo {
    pub revision: u64,
    pub committer: String,
    pub date: DateTime<FixedOffset>,
    pub message: String,
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
