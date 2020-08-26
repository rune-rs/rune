use crate::vm::inst;
use std::fmt;

pub trait BoxedPanic: 'static + fmt::Display + fmt::Debug + Send + Sync {}
impl<T> BoxedPanic for T where T: 'static + fmt::Display + fmt::Debug + Send + Sync {}

/// A descriptibe panic.
///
/// This can be used as an error variant in functions that you want to be able
/// to panic.
#[derive(Debug)]
pub struct Panic {
    inner: Box<dyn BoxedPanic>,
}

impl Panic {
    /// A panic from a message.
    pub fn msg<D>(message: D) -> Self
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

impl From<inst::PanicReason> for Panic {
    fn from(value: inst::PanicReason) -> Self {
        Self {
            inner: Box::new(value),
        }
    }
}
