use core::ptr::{self};
use core::slice::{self};

use crate::alloc::Allocator;
use crate::error::Error;

use super::{Drain, Vec};

// NB: This is a larger rewrite than typical, but that's because the `Splice`
// does a lot of work when it's dropped instead of performing the work in-place
// like this.
pub(crate) fn splice<'a, I, A>(
    drain: &mut Drain<'a, I::Item, A>,
    replace_with: &mut I,
) -> Result<(), Error>
where
    I: Iterator + 'a,
    A: Allocator + 'a,
{
    for element in drain.by_ref() {
        drop(element);
    }

    // At this point draining is done and the only remaining tasks are splicing
    // and moving things into the final place.
    // Which means we can replace the slice::Iter with pointers that won't point to deallocated
    // memory, so that Drain::drop is still allowed to call iter.len(), otherwise it would break
    // the ptr.sub_ptr contract.
    drain.iter = [].iter();

    unsafe {
        if drain.tail_len == 0 {
            let out = drain.vec.as_mut();

            for element in replace_with.by_ref() {
                out.try_push(element)?;
            }

            return Ok(());
        }

        // First fill the range left by drain().
        if !drain.fill(replace_with) {
            return Ok(());
        }

        // There may be more elements. Use the lower bound as an estimate.
        // FIXME: Is the upper bound a better guess? Or something else?
        let (lower_bound, _upper_bound) = replace_with.size_hint();

        if lower_bound > 0 {
            drain.move_tail(lower_bound)?;

            if !drain.fill(replace_with) {
                return Ok(());
            }
        }

        // Collect any remaining elements.
        // This is a zero-length vector which does not allocate if `lower_bound` was exact.
        let mut collected = Vec::new_in(drain.vec.as_ref().allocator());

        for element in replace_with.by_ref() {
            collected.try_push(element)?;
        }

        let mut collected = collected.into_iter();

        // Now we have an exact count.
        if collected.len() > 0 {
            drain.move_tail(collected.len())?;
            let filled = drain.fill(&mut collected);
            debug_assert!(filled);
            debug_assert_eq!(collected.len(), 0);
        }

        Ok(())
    }
    // Let `Drain::drop` move the tail back if necessary and restore `vec.len`.
}

/// Private helper methods for `Splice::drop`
impl<T, A: Allocator> Drain<'_, T, A> {
    /// The range from `self.vec.len` to `self.tail_start` contains elements
    /// that have been moved out.
    /// Fill that range as much as possible with new elements from the `replace_with` iterator.
    /// Returns `true` if we filled the entire range. (`replace_with.next()` didn’t return `None`.)
    unsafe fn fill<I: Iterator<Item = T>>(&mut self, replace_with: &mut I) -> bool {
        let vec = unsafe { self.vec.as_mut() };
        let range_start = vec.len;
        let range_end = self.tail_start;
        let range_slice = unsafe {
            slice::from_raw_parts_mut(vec.as_mut_ptr().add(range_start), range_end - range_start)
        };

        for place in range_slice {
            if let Some(new_item) = replace_with.next() {
                unsafe { ptr::write(place, new_item) };
                vec.len += 1;
            } else {
                return false;
            }
        }
        true
    }

    /// Makes room for inserting more elements before the tail.
    unsafe fn move_tail(&mut self, additional: usize) -> Result<(), Error> {
        let vec = unsafe { self.vec.as_mut() };
        let len = self.tail_start + self.tail_len;
        vec.buf.try_reserve(len, additional)?;

        let new_tail_start = self.tail_start + additional;
        unsafe {
            let src = vec.as_ptr().add(self.tail_start);
            let dst = vec.as_mut_ptr().add(new_tail_start);
            ptr::copy(src, dst, self.tail_len);
        }
        self.tail_start = new_tail_start;
        Ok(())
    }
}
