use crate::alloc::Vec;
use crate::runtime::Args;
use crate::runtime::{Stack, UnsafeToValue, Value, VmResult};
use crate::vm_try;

/// Trait for converting arguments onto the stack.
///
/// This can take references, because it is unsafe to call. And should only be
/// implemented in contexts where it can be guaranteed that the references will
/// not outlive the call.
pub trait GuardedArgs {
    /// Guard that when dropped will invalidate any values encoded.
    type Guard;

    /// Encode arguments onto a stack.
    ///
    /// # Safety
    ///
    /// This can encode references onto the stack. The caller must ensure that
    /// the guard is dropped before any references which have been encoded are
    /// no longer alive.
    unsafe fn guarded_into_stack(self, stack: &mut Stack) -> VmResult<Self::Guard>;

    /// Encode arguments into a vector.
    ///
    /// # Safety
    ///
    /// This can encode references into the vector. The caller must ensure that
    /// the guard is dropped before any references which have been encoded are
    /// no longer alive.
    unsafe fn guarded_into_vec(self) -> VmResult<(Vec<Value>, Self::Guard)>;

    /// The number of arguments.
    fn count(&self) -> usize;
}

macro_rules! impl_into_args {
    ($count:expr $(, $ty:ident $value:ident $_:expr)*) => {
        impl<$($ty,)*> GuardedArgs for ($($ty,)*)
        where
            $($ty: UnsafeToValue,)*
        {
            type Guard = ($($ty::Guard,)*);

            #[allow(unused)]
            #[inline]
            unsafe fn guarded_into_stack(self, stack: &mut Stack) -> VmResult<Self::Guard> {
                let ($($value,)*) = self;
                $(let $value = $crate::vm_try!($value.unsafe_to_value());)*
                $($crate::vm_try!(stack.push($value.0));)*
                VmResult::Ok(($($value.1,)*))
            }

            #[allow(unused)]
            #[inline]
            unsafe fn guarded_into_vec(self) -> VmResult<(Vec<Value>, Self::Guard)> {
                let ($($value,)*) = self;
                $(let $value = $crate::vm_try!($value.unsafe_to_value());)*
                let mut out = $crate::vm_try!(Vec::try_with_capacity($count));
                $($crate::vm_try!(out.try_push($value.0));)*
                VmResult::Ok((out, ($($value.1,)*)))
            }

            #[inline]
            fn count(&self) -> usize {
                $count
            }
        }
    };
}

repeat_macro!(impl_into_args);

impl GuardedArgs for Vec<Value> {
    type Guard = ();

    #[inline]
    unsafe fn guarded_into_stack(self, stack: &mut Stack) -> VmResult<Self::Guard> {
        self.into_stack(stack)
    }

    #[inline]
    unsafe fn guarded_into_vec(self) -> VmResult<(Vec<Value>, Self::Guard)> {
        VmResult::Ok((self, ()))
    }

    #[inline]
    fn count(&self) -> usize {
        (self as &dyn Args).count()
    }
}

impl GuardedArgs for rust_alloc::vec::Vec<Value> {
    type Guard = ();

    #[inline]
    unsafe fn guarded_into_stack(self, stack: &mut Stack) -> VmResult<Self::Guard> {
        self.into_stack(stack)
    }

    #[inline]
    unsafe fn guarded_into_vec(self) -> VmResult<(Vec<Value>, Self::Guard)> {
        VmResult::Ok((vm_try!(Vec::try_from(self)), ()))
    }

    #[inline]
    fn count(&self) -> usize {
        self.len()
    }
}
