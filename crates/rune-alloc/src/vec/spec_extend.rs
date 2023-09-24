#[cfg(rune_nightly)]
use core::slice;

use crate::alloc::Allocator;
#[cfg(rune_nightly)]
use crate::clone::{TryClone, TryCopy};
use crate::error::Error;

#[cfg(rune_nightly)]
use super::IntoIter;
use super::Vec;

// Specialization trait used for Vec::extend
pub(super) trait SpecExtend<T, I> {
    fn spec_extend(&mut self, iter: I) -> Result<(), Error>;
}

impl<T, I, A: Allocator> SpecExtend<T, I> for Vec<T, A>
where
    I: Iterator<Item = T>,
{
    default_fn! {
        fn spec_extend(&mut self, iter: I) -> Result<(), Error> {
            for value in iter {
                self.try_push(value)?;
            }

            Ok(())
        }
    }
}

#[cfg(rune_nightly)]
impl<T, A: Allocator> SpecExtend<T, IntoIter<T>> for Vec<T, A> {
    fn spec_extend(&mut self, mut iterator: IntoIter<T>) -> Result<(), Error> {
        unsafe {
            self.try_append_elements(iterator.as_slice() as _)?;
        }
        iterator.forget_remaining_elements();
        Ok(())
    }
}

#[cfg(rune_nightly)]
impl<'a, T: 'a, I, A: Allocator> SpecExtend<&'a T, I> for Vec<T, A>
where
    I: Iterator<Item = &'a T>,
    T: TryClone,
{
    default fn spec_extend(&mut self, iterator: I) -> Result<(), Error> {
        for value in iterator {
            self.try_push(value.try_clone()?)?;
        }

        Ok(())
    }
}

#[cfg(rune_nightly)]
impl<'a, T: 'a, A: Allocator> SpecExtend<&'a T, slice::Iter<'a, T>> for Vec<T, A>
where
    T: TryCopy,
{
    fn spec_extend(&mut self, iterator: slice::Iter<'a, T>) -> Result<(), Error> {
        let slice = iterator.as_slice();

        unsafe {
            self.try_append_elements(slice)?;
        }

        Ok(())
    }
}
