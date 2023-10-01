use core::fmt;
use core::ops::Range;

use crate::alloc;
use crate::compile;

#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum FormattingError {
    BadRange(Range<usize>, usize),
    Compile(compile::Error),
    Alloc(alloc::Error),
    OpenComment,
}

impl fmt::Display for FormattingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FormattingError::BadRange(range, length) => {
                write!(f, "Range {range:?} is not within 0-{length}")
            }
            FormattingError::Compile(error) => error.fmt(f),
            FormattingError::Alloc(error) => error.fmt(f),
            FormattingError::OpenComment {} => write!(f, "Expected closing of comment"),
        }
    }
}

impl From<compile::Error> for FormattingError {
    #[inline]
    fn from(error: compile::Error) -> Self {
        FormattingError::Compile(error)
    }
}

impl From<alloc::Error> for FormattingError {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        FormattingError::Alloc(error)
    }
}

cfg_std! {
    impl std::error::Error for FormattingError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                FormattingError::Compile(error) => Some(error),
                FormattingError::Alloc(error) => Some(error),
                _ => None,
            }
        }
    }
}
