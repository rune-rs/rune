use rune::alloc::fmt::TryWrite;
use rune::runtime::{Formatter, VmResult};
use rune::{vm_write, Any};

/// An os error returned by methods in the `rand` module.
#[derive(Debug, Any)]
#[rune(item = ::rand)]
pub(super) struct OsError {
    pub(super) inner: rand::rand_core::OsError,
}

impl From<rand::rand_core::OsError> for OsError {
    #[inline]
    fn from(inner: rand::rand_core::OsError) -> Self {
        Self { inner }
    }
}

impl OsError {
    /// Write a display representation the error.
    #[rune::function(instance, protocol = DISPLAY_FMT)]
    fn display_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{}", self.inner)
    }
}
