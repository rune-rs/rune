use crate::alloc::Vec;
use crate::runtime::Args;
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

    /// Attempts to convert this type into Args, which will only succeed as long
    /// as it doesn't contain any references to Any types.
    fn try_into_args(self) -> Option<impl Args>;

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

            fn try_into_args(self) -> Option<impl Args> {
                let ($($value,)*) = &self;
                $(if !$value.is_to_value() { return None })*
                let ($($value,)*) = self;
                Some(($($value.try_into_to_value()?,)*))
            }

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
    unsafe fn unsafe_into_stack(self, stack: &mut Stack) -> VmResult<Self::Guard> {
        self.into_stack(stack)
    }

    fn try_into_args(self) -> Option<impl Args> {
        Some(self)
    }

    #[inline]
    fn count(&self) -> usize {
        (self as &dyn Args).count()
    }
}

#[cfg(feature = "alloc")]
impl GuardedArgs for ::rust_alloc::vec::Vec<Value> {
    type Guard = ();

    #[inline]
    unsafe fn unsafe_into_stack(self, stack: &mut Stack) -> VmResult<Self::Guard> {
        self.into_stack(stack)
    }

    fn try_into_args(self) -> Option<impl Args> {
        Some(self)
    }

    #[inline]
    fn count(&self) -> usize {
        (self as &dyn Args).count()
    }
}
