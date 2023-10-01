use crate::alloc::Allocator;
use crate::error::Error;
use crate::vec::Vec;

use super::TryWrite;

/// [`TryWrite`] is implemented for `Vec<u8>` by appending to the vector. The
/// vector will grow as needed.
impl<A: Allocator> TryWrite for Vec<u8, A> {
    #[inline]
    fn try_write_str(&mut self, s: &str) -> Result<(), Error> {
        self.try_extend_from_slice(s.as_bytes())
    }
}
