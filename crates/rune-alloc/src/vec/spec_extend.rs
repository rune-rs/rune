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

impl<T, I, A> SpecExtend<T, I> for Vec<T, A>
where
    I: Iterator<Item = T>,
    A: Allocator,
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
impl<T, A> SpecExtend<T, IntoIter<T>> for Vec<T, A>
where
    A: Allocator,
{
    fn spec_extend(&mut self, mut iterator: IntoIter<T>) -> Result<(), Error> {
        unsafe {
            self.try_append_elements(iterator.as_slice() as _)?;
        }
        iterator.forget_remaining_elements();
        Ok(())
    }
}

#[cfg(rune_nightly)]
impl<'a, T, I, A> SpecExtend<&'a T, I> for Vec<T, A>
where
    I: Iterator<Item = &'a T>,
    T: 'a + TryClone,
    A: Allocator,
{
    default fn spec_extend(&mut self, iterator: I) -> Result<(), Error> {
        for value in iterator {
            self.try_push(value.try_clone()?)?;
        }

        Ok(())
    }
}

#[cfg(rune_nightly)]
impl<'a, T, A> SpecExtend<&'a T, slice::Iter<'a, T>> for Vec<T, A>
where
    T: TryCopy,
    T: 'a,
    A: Allocator,
{
    fn spec_extend(&mut self, iterator: slice::Iter<'a, T>) -> Result<(), Error> {
        let slice = iterator.as_slice();

        unsafe {
            self.try_append_elements(slice)?;
        }

        Ok(())
    }
}
