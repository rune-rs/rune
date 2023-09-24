use core::fmt;

use crate::no_std::io;

use crate::alloc;
use crate::compile;

#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum FormattingError {
    Io(io::Error),
    InvalidSpan(usize, usize, usize),
    CompileError(compile::Error),
    AllocError(alloc::Error),
    Eof,
}

impl fmt::Display for FormattingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FormattingError::Io(error) => error.fmt(f),
            FormattingError::InvalidSpan(from, to, max) => {
                write!(f, "Invalid span {from}..{to} but max is {max}",)
            }
            FormattingError::CompileError(error) => error.fmt(f),
            FormattingError::AllocError(error) => error.fmt(f),
            FormattingError::Eof {} => write!(f, "Unexpected end of input"),
        }
    }
}

impl From<io::Error> for FormattingError {
    #[inline]
    fn from(error: io::Error) -> Self {
        FormattingError::Io(error)
    }
}

impl From<compile::Error> for FormattingError {
    #[inline]
    fn from(error: compile::Error) -> Self {
        FormattingError::CompileError(error)
    }
}

impl From<alloc::Error> for FormattingError {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        FormattingError::AllocError(error)
    }
}

impl crate::no_std::error::Error for FormattingError {
    fn source(&self) -> Option<&(dyn crate::no_std::error::Error + 'static)> {
        match self {
            FormattingError::Io(error) => Some(error),
            FormattingError::CompileError(error) => Some(error),
            _ => None,
        }
    }
}
