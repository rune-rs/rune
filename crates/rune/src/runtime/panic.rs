use crate::runtime::PanicReason;
use std::fmt;

pub trait BoxedPanic: 'static + fmt::Display + fmt::Debug + Send + Sync {}
impl<T> BoxedPanic for T where T: 'static + fmt::Display + fmt::Debug + Send + Sync {}

/// A descriptive panic.
///
/// This can be used as an error variant in native functions that you want to be
/// able to panic.
#[derive(Debug)]
pub struct Panic {
    inner: Box<dyn BoxedPanic>,
}

impl Panic {
    /// A custom panic reason.
    pub(crate) fn custom<D>(message: D) -> Self
    where
        D: BoxedPanic,
    {
        Self {
            inner: Box::new(message),
        }
    }
}

impl fmt::Display for Panic {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.inner)
    }
}

impl From<PanicReason> for Panic {
    fn from(value: PanicReason) -> Self {
        Self {
            inner: Box::new(value),
        }
    }
}
