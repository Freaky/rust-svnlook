use std::fmt;

use crate::{SvnError, SvnStatus};

impl fmt::Display for SvnError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SvnError::CommandError(io) => io.fmt(f),
            SvnError::ExitFailure(status) => write!(f, "non-zero exit from command: {}", status),
            SvnError::ParseError => write!(f, "parse error"),
        }
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
