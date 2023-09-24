#[cfg(rune_nightly)]
use core::ptr;

use crate::alloc::Allocator;
use crate::clone::TryClone;
use crate::error::Error;
#[cfg(rune_nightly)]
use crate::raw_vec::RawVec;

#[cfg(rune_nightly)]
use super::IsZero;
use super::Vec;

// Specialization trait used for Vec::from_elem
pub(super) trait SpecFromElem: Sized {
    fn from_elem<A: Allocator>(elem: Self, n: usize, alloc: A) -> Result<Vec<Self, A>, Error>;
}

impl<T> SpecFromElem for T
where
    T: TryClone,
{
    default_fn! {
        fn from_elem<A: Allocator>(elem: Self, n: usize, alloc: A) -> Result<Vec<Self, A>, Error> {
            let mut v = Vec::try_with_capacity_in(n, alloc)?;
            v.try_extend_with(n, elem)?;
            Ok(v)
        }
    }
}

#[cfg(rune_nightly)]
impl<T> SpecFromElem for T
where
    T: TryClone + IsZero,
{
    #[inline]
    default fn from_elem<A: Allocator>(elem: T, n: usize, alloc: A) -> Result<Vec<T, A>, Error> {
        if elem.is_zero() {
            return Ok(Vec {
                buf: RawVec::try_with_capacity_zeroed_in(n, alloc)?,
                len: n,
            });
        }

        let mut v = Vec::try_with_capacity_in(n, alloc)?;
        v.try_extend_with(n, elem)?;
        Ok(v)
    }
}

#[cfg(rune_nightly)]
impl SpecFromElem for i8 {
    #[inline]
    fn from_elem<A: Allocator>(elem: i8, n: usize, alloc: A) -> Result<Vec<i8, A>, Error> {
        if elem == 0 {
            return Ok(Vec {
                buf: RawVec::try_with_capacity_zeroed_in(n, alloc)?,
                len: n,
            });
        }

        unsafe {
            let mut v = Vec::try_with_capacity_in(n, alloc)?;
            ptr::write_bytes(v.as_mut_ptr(), elem as u8, n);
            v.set_len(n);
            Ok(v)
        }
    }
}

#[cfg(rune_nightly)]
impl SpecFromElem for u8 {
    #[inline]
    fn from_elem<A: Allocator>(elem: u8, n: usize, alloc: A) -> Result<Vec<u8, A>, Error> {
        if elem == 0 {
            return Ok(Vec {
                buf: RawVec::try_with_capacity_zeroed_in(n, alloc)?,
                len: n,
            });
        }

        unsafe {
            let mut v = Vec::try_with_capacity_in(n, alloc)?;
            ptr::write_bytes(v.as_mut_ptr(), elem, n);
            v.set_len(n);
            Ok(v)
        }
    }
}
