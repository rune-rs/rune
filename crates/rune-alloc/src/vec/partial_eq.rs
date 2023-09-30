use crate::alloc::Allocator;

use super::Vec;

macro_rules! __impl_slice_eq1 {
    ([$($vars:tt)*] $lhs:ty, $rhs:ty $(where $ty:ty: $bound:ident)?) => {
        impl<T, U, $($vars)*> PartialEq<$rhs> for $lhs
        where
            T: PartialEq<U>,
            $($ty: $bound)?
        {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool { self[..] == other[..] }
            #[inline]
            #[allow(clippy::partialeq_ne_impl)]
            fn ne(&self, other: &$rhs) -> bool { self[..] != other[..] }
        }
    }
}

#[cfg(feature = "alloc")]
__impl_slice_eq1! { [A: Allocator] Vec<T, A>, ::rust_alloc::vec::Vec<U> }
#[cfg(feature = "alloc")]
__impl_slice_eq1! { [A: Allocator] ::rust_alloc::vec::Vec<T>, Vec<U, A> }
__impl_slice_eq1! { [A1: Allocator, A2: Allocator] Vec<T, A1>, Vec<U, A2> }
__impl_slice_eq1! { [A: Allocator] Vec<T, A>, &[U] }
__impl_slice_eq1! { [A: Allocator] Vec<T, A>, &mut [U] }
__impl_slice_eq1! { [A: Allocator] &[T], Vec<U, A> }
__impl_slice_eq1! { [A: Allocator] &mut [T], Vec<U, A> }
__impl_slice_eq1! { [A: Allocator] Vec<T, A>, [U] }
__impl_slice_eq1! { [A: Allocator] [T], Vec<U, A> }
__impl_slice_eq1! { [A: Allocator, const N: usize] Vec<T, A>, [U; N] }
__impl_slice_eq1! { [A: Allocator, const N: usize] Vec<T, A>, &[U; N] }
