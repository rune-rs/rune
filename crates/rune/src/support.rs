//! This module is primarily provided to support test cases and examples. It is
//! not intended for end-users and might change at any time.

#[doc(inline)]
#[cfg(feature = "anyhow")]
pub use anyhow::Context;

#[cfg(not(feature = "std"))]
#[doc(inline)]
pub use self::no_std::{Error, Result};
#[cfg(feature = "std")]
#[doc(inline)]
pub use anyhow::{Error, Result};

#[cfg(not(feature = "std"))]
pub(crate) mod no_std {
    use core::fmt;

    use crate::alloc;
    use crate::build;
    use crate::compile;
    use crate::runtime;
    #[cfg(test)]
    use crate::tests;

    /// Type alias for for results which uses [`Error`] by default.
    ///
    /// For errors which aren't automatically captures, you should map them
    /// using [`Error::msg`].
    pub type Result<T, E = Error> = core::result::Result<T, E>;

    /// Error kind which supports capturing any toplevel errors produced by
    /// Rune.
    #[derive(Debug)]
    pub struct Error {
        kind: ErrorKind,
    }

    impl Error {
        /// Create a new error object from a printable error message.
        #[cfg(feature = "anyhow")]
        pub fn msg<M>(message: M) -> Self
        where
            M: fmt::Display + fmt::Debug + Send + Sync + 'static,
        {
            Self {
                kind: ErrorKind::Custom(anyhow::Error::msg(message)),
            }
        }

        /// Create a new error object from a printable error message.
        #[cfg(not(feature = "anyhow"))]
        pub fn msg<M>(message: M) -> Self
        where
            M: fmt::Display + fmt::Debug + Send + Sync + 'static,
        {
            match crate::alloc::fmt::try_format(format_args!("{message}")) {
                Ok(string) => Self {
                    kind: ErrorKind::Custom(string),
                },
                Err(error) => Self {
                    kind: ErrorKind::Alloc(error),
                },
            }
        }
    }

    impl From<alloc::Error> for Error {
        #[inline]
        fn from(error: alloc::Error) -> Self {
            Self {
                kind: ErrorKind::Alloc(error),
            }
        }
    }

    impl From<compile::ContextError> for Error {
        #[inline]
        fn from(error: compile::ContextError) -> Self {
            Self {
                kind: ErrorKind::Context(error),
            }
        }
    }

    impl From<compile::Error> for Error {
        #[inline]
        fn from(error: compile::Error) -> Self {
            Self {
                kind: ErrorKind::Compile(error),
            }
        }
    }

    impl From<build::BuildError> for Error {
        #[inline]
        fn from(error: build::BuildError) -> Self {
            Self {
                kind: ErrorKind::Build(error),
            }
        }
    }

    impl From<runtime::VmError> for Error {
        #[inline]
        fn from(error: runtime::VmError) -> Self {
            Self {
                kind: ErrorKind::Vm(error),
            }
        }
    }

    impl From<runtime::RuntimeError> for Error {
        #[inline]
        fn from(error: runtime::RuntimeError) -> Self {
            Self {
                kind: ErrorKind::Runtime(error),
            }
        }
    }

    #[cfg(feature = "anyhow")]
    impl From<anyhow::Error> for Error {
        #[inline]
        fn from(error: anyhow::Error) -> Self {
            Self {
                kind: ErrorKind::Custom(error),
            }
        }
    }

    #[cfg(test)]
    impl From<tests::TestError> for Error {
        #[inline]
        fn from(error: tests::TestError) -> Self {
            Self {
                kind: ErrorKind::Test(error),
            }
        }
    }

    impl fmt::Display for Error {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match &self.kind {
                ErrorKind::Alloc(error) => error.fmt(f),
                ErrorKind::Context(error) => error.fmt(f),
                ErrorKind::Compile(error) => error.fmt(f),
                ErrorKind::Build(error) => error.fmt(f),
                ErrorKind::Runtime(error) => error.fmt(f),
                ErrorKind::Vm(error) => error.fmt(f),
                ErrorKind::Custom(error) => error.fmt(f),
                #[cfg(test)]
                ErrorKind::Test(error) => error.fmt(f),
            }
        }
    }

    #[derive(Debug)]
    enum ErrorKind {
        Alloc(alloc::Error),
        Context(compile::ContextError),
        Compile(compile::Error),
        Build(build::BuildError),
        Vm(runtime::VmError),
        Runtime(runtime::RuntimeError),
        #[cfg(feature = "anyhow")]
        Custom(anyhow::Error),
        #[cfg(not(feature = "anyhow"))]
        Custom(alloc::String),
        #[cfg(test)]
        Test(tests::TestError),
    }

    impl core::error::Error for Error {
        fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
            match &self.kind {
                ErrorKind::Alloc(error) => Some(error),
                ErrorKind::Context(error) => Some(error),
                ErrorKind::Compile(error) => Some(error),
                ErrorKind::Build(error) => Some(error),
                ErrorKind::Vm(error) => Some(error),
                ErrorKind::Runtime(error) => Some(error),
                #[cfg(feature = "anyhow")]
                ErrorKind::Custom(error) => Some(error.as_ref()),
                #[cfg(not(feature = "anyhow"))]
                ErrorKind::Custom(..) => None,
                #[cfg(test)]
                ErrorKind::Test(error) => Some(error),
            }
        }
    }
}
