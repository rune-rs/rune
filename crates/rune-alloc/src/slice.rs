pub(crate) use self::iter::{RawIter, RawIterMut};
pub(crate) mod iter;

cfg_if! {
    if #[cfg(rune_nightly)] {
        pub(crate) use core::slice::range;
    } else {
        use core::ops;

        #[must_use]
        pub(crate) fn range<R>(range: R, bounds: ops::RangeTo<usize>) -> ops::Range<usize>
        where
            R: ops::RangeBounds<usize>,
        {
            let len = bounds.end;

            let start: ops::Bound<&usize> = range.start_bound();
            let start = match start {
                ops::Bound::Included(&start) => start,
                ops::Bound::Excluded(start) => start
                    .checked_add(1)
                    .unwrap_or_else(|| slice_start_index_overflow_fail()),
                ops::Bound::Unbounded => 0,
            };

            let end: ops::Bound<&usize> = range.end_bound();
            let end = match end {
                ops::Bound::Included(end) => end
                    .checked_add(1)
                    .unwrap_or_else(|| slice_end_index_overflow_fail()),
                ops::Bound::Excluded(&end) => end,
                ops::Bound::Unbounded => len,
            };

            if start > end {
                slice_index_order_fail(start, end);
            }
            if end > len {
                slice_end_index_len_fail(end, len);
            }

            ops::Range { start, end }
        }

        const fn slice_start_index_overflow_fail() -> ! {
            panic!("attempted to index slice from after maximum usize");
        }

        const fn slice_end_index_overflow_fail() -> ! {
            panic!("attempted to index slice up to maximum usize");
        }

        fn slice_index_order_fail(index: usize, end: usize) -> ! {
            panic!("slice index starts at {index} but ends at {end}");
        }

        fn slice_end_index_len_fail(index: usize, len: usize) -> ! {
            panic!("range end index {index} out of range for slice of length {len}");
        }
    }
}
