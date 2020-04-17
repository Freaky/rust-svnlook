use std::{error::Error, io};

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
