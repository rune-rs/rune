use crate::alloc::Vec;
use crate::runtime::{Stack, UnsafeToValue, Value, VmResult};

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
    /// This is implemented for and allows encoding references on the stack.
    /// The returned guard must be dropped before any used references are
    /// invalidated.
    unsafe fn unsafe_into_stack(self, stack: &mut Stack) -> VmResult<Self::Guard>;

    /// Convert arguments into a vector.
    ///
    /// # Safety
    ///
    /// This is implemented for and allows encoding references on the stack.
    /// The returned guard must be dropped before any used references are
    /// invalidated.
    unsafe fn unsafe_into_vec(self) -> VmResult<(Vec<Value>, Self::Guard)>;

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
            unsafe fn unsafe_into_stack(self, stack: &mut Stack) -> VmResult<Self::Guard> {
                let ($($value,)*) = self;
                $(let $value = vm_try!($value.unsafe_to_value());)*
                $(vm_try!(stack.push($value.0));)*
                VmResult::Ok(($($value.1,)*))
            }

            #[allow(unused)]
            unsafe fn unsafe_into_vec(self) -> VmResult<(Vec<Value>, Self::Guard)> {
                let ($($value,)*) = self;
                let mut vec = vm_try!(Vec::try_with_capacity($count));
                $(let $value = vm_try!($value.unsafe_to_value());)*
                $(vm_try!(vec.try_push($value.0));)*
                VmResult::Ok((vec, ($($value.1,)*)))
            }

            fn count(&self) -> usize {
                $count
            }
        }
    };
}

repeat_macro!(impl_into_args);
