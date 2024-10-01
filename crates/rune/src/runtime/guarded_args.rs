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

impl GuardedArgs for Vec<Value> {
    type Guard = ();

    #[inline]
    unsafe fn unsafe_into_stack(self, stack: &mut Stack) -> VmResult<Self::Guard> {
        self.into_stack(stack)
    }

    #[inline]
    unsafe fn unsafe_into_vec(self) -> VmResult<(Vec<Value>, Self::Guard)> {
        VmResult::Ok((vm_try!(self.try_into_vec()), ()))
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

    #[inline]
    unsafe fn unsafe_into_vec(self) -> VmResult<(Vec<Value>, Self::Guard)> {
        VmResult::Ok((vm_try!(self.try_into_vec()), ()))
    }

    #[inline]
    fn count(&self) -> usize {
        (self as &dyn Args).count()
    }
}

#[cfg(test)]
mod tests {
    use super::GuardedArgs;
    use crate::Value;

    #[derive(Default)]
    struct MyAny {}

    crate::__internal_impl_any!(self, MyAny);

    fn get_guarded_arg_value() -> impl GuardedArgs {
        (Value::unit(),)
    }

    fn get_guarded_arg_reference(value: &MyAny) -> impl GuardedArgs {
        let (by_reference, _) = unsafe { Value::from_ref(value) }.unwrap();
        (by_reference,)
    }

    fn get_guarded_arg_rune_vec() -> impl GuardedArgs {
        let by_value = Value::unit();

        let mine = MyAny::default();
        let (by_reference, _) = unsafe { Value::from_ref(&mine) }.unwrap();

        let mut values = crate::alloc::Vec::new();
        values.try_push(by_value).unwrap();
        values.try_push(by_reference).unwrap();
        values
    }

    #[test]
    fn assert_references_are_not_readable() {
        let (value_result, _) = unsafe { get_guarded_arg_value().unsafe_into_vec() }.unwrap();
        assert_eq!(value_result.len(), 1);
        assert!(value_result[0].is_readable());

        let mine = MyAny::default();
        let (reference_result, _) =
            unsafe { get_guarded_arg_reference(&mine).unsafe_into_vec() }.unwrap();
        assert_eq!(value_result.len(), 1);
        assert!(!reference_result[0].is_readable());

        let rune_vec = get_guarded_arg_rune_vec();
        let (rune_vec_result, _) = unsafe { rune_vec.unsafe_into_vec() }.unwrap();
        assert_eq!(rune_vec_result.len(), 2);
        assert!(rune_vec_result[0].is_readable());
        assert!(!rune_vec_result[1].is_readable());
    }

    #[test]
    fn assert_references_are_not_writable() {
        let (value_result, _) = unsafe { get_guarded_arg_value().unsafe_into_vec() }.unwrap();
        assert_eq!(value_result.len(), 1);
        assert!(value_result[0].is_writable());

        let mine = MyAny::default();
        let (reference_result, _) =
            unsafe { get_guarded_arg_reference(&mine).unsafe_into_vec() }.unwrap();
        assert_eq!(value_result.len(), 1);
        assert!(!reference_result[0].is_writable());

        let rune_vec = get_guarded_arg_rune_vec();
        let (rune_vec_result, _) = unsafe { rune_vec.unsafe_into_vec() }.unwrap();
        assert_eq!(rune_vec_result.len(), 2);
        assert!(rune_vec_result[0].is_writable());
        assert!(!rune_vec_result[1].is_writable());
    }

    #[cfg(feature = "std")]
    fn get_guarded_arg_std_vec() -> impl GuardedArgs {
        let by_value = Value::unit();

        let mine = MyAny::default();
        let (by_reference, _) = unsafe { Value::from_ref(&mine) }.unwrap();

        vec![by_value, by_reference]
    }

    #[cfg(feature = "std")]
    #[test]
    fn assert_references_are_not_readable_std() {
        let std_vec = get_guarded_arg_std_vec();
        let (std_vec_result, _) = unsafe { std_vec.unsafe_into_vec() }.unwrap();
        assert_eq!(std_vec_result.len(), 2);
        assert!(std_vec_result[0].is_readable());
        assert!(!std_vec_result[1].is_readable());
    }

    #[cfg(feature = "std")]
    #[test]
    fn assert_references_are_not_writable_std() {
        let std_vec = get_guarded_arg_std_vec();
        let (std_vec_result, _) = unsafe { std_vec.unsafe_into_vec() }.unwrap();
        assert_eq!(std_vec_result.len(), 2);
        assert!(std_vec_result[0].is_writable());
        assert!(!std_vec_result[1].is_writable());
    }
}
