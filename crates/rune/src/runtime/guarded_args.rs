use crate::runtime::{Stack, UnsafeToValue, VmError};

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
    unsafe fn unsafe_into_stack(self, stack: &mut Stack) -> Result<Self::Guard, VmError>;

    /// The number of arguments.
    fn count(&self) -> usize;
}

macro_rules! impl_into_args {
    () => {
        impl_into_args!{@impl 0,}
    };

    ({$ty:ident, $value:ident, $count:expr}, $({$l_ty:ident, $l_value:ident, $l_count:expr},)*) => {
        impl_into_args!{@impl $count, {$ty, $value, $count}, $({$l_ty, $l_value, $l_count},)*}
        impl_into_args!{$({$l_ty, $l_value, $l_count},)*}
    };

    (@impl $count:expr, $({$ty:ident, $value:ident, $ignore_count:expr},)*) => {
        impl<$($ty,)*> GuardedArgs for ($($ty,)*)
        where
            $($ty: UnsafeToValue,)*
        {
            type Guard = ($($ty::Guard,)*);

            #[allow(unused)]
            unsafe fn unsafe_into_stack(self, stack: &mut Stack) -> Result<Self::Guard, VmError> {
                let ($($value,)*) = self;
                $(let $value = $value.unsafe_to_value()?;)*
                $(stack.push($value.0);)*
                Ok(($($value.1,)*))
            }

            fn count(&self) -> usize {
                $count
            }
        }
    };
}

repeat_macro!(impl_into_args);
