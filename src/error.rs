use std::{error::Error, io, fmt};

#[derive(Debug)]
pub enum SvnError {
    CommandError(io::Error),
    ExitFailure(std::process::ExitStatus),
    ParseError,
}

impl Error for SvnError {}

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

impl fmt::Display for SvnError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SvnError::CommandError(io) => io.fmt(f),
            SvnError::ExitFailure(status) => write!(f, "non-zero exit from command: {}", status),
            SvnError::ParseError => write!(f, "parse error"),
        }
    }
}