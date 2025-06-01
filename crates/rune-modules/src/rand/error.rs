use rune::alloc::fmt::TryWrite;
use rune::runtime::{Formatter, VmResult};
use rune::{vm_write, Any};

/// An error returned by methods in the `rand` module.
#[derive(Debug, Any)]
#[rune(item = ::rand)]
pub(super) struct Error {
    pub(super) inner: getrandom::Error,
}

impl From<getrandom::Error> for Error {
    #[inline]
    fn from(inner: getrandom::Error) -> Self {
        Self { inner }
    }
}

impl Error {
    /// Write a display representation the error.
    #[rune::function(instance, protocol = DISPLAY_FMT)]
    fn display_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{}", self.inner)
    }
}
