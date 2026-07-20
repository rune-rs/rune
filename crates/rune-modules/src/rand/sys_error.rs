use rune::alloc;
use rune::alloc::fmt::TryWrite;
use rune::runtime::Formatter;
use rune::Any;

/// An os error returned by methods in the `rand` module.
#[derive(Debug, Any)]
#[rune(item = ::rand)]
pub(super) struct SysError {
    pub(super) inner: rand::rngs::SysError,
}

impl From<rand::rngs::SysError> for SysError {
    #[inline]
    fn from(inner: rand::rngs::SysError) -> Self {
        Self { inner }
    }
}

impl SysError {
    /// Write a display representation the error.
    #[rune::function(instance, protocol = DISPLAY_FMT)]
    fn display_fmt(&self, f: &mut Formatter) -> alloc::Result<()> {
        write!(f, "{}", self.inner)
    }
}
