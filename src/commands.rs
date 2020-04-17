
pub(crate) mod diff;
pub(crate) mod changed;
pub(crate) mod info;

pub use diff::*;
pub use changed::*;
pub use info::*;

use crate::SvnError;

pub(crate) fn try_chomp(slice: &[u8]) -> Result<&[u8], SvnError> {
    if slice.ends_with(b"\n") {
        Ok(&slice[..slice.len() - 1])
    } else {
        Err(SvnError::ParseError)
    }
}
