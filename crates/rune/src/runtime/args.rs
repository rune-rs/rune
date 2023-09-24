use crate::alloc::Vec;
use crate::runtime::{Stack, ToValue, Value, VmResult};

/// Trait for converting arguments onto the stack.
pub trait Args {
    /// Encode arguments onto a stack.
    fn into_stack(self, stack: &mut Stack) -> VmResult<()>;

    /// Convert arguments into a vector.
    fn try_into_vec(self) -> VmResult<Vec<Value>>;

    /// The number of arguments.
    fn count(&self) -> usize;
}

macro_rules! impl_into_args {
    ($count:expr $(, $ty:ident $value:ident $_:expr)*) => {
        impl<$($ty,)*> Args for ($($ty,)*)
        where
            $($ty: ToValue,)*
        {
            #[allow(unused)]
            fn into_stack(self, stack: &mut Stack) -> VmResult<()> {
                let ($($value,)*) = self;
                $(vm_try!(stack.push(vm_try!($value.to_value())));)*
                VmResult::Ok(())
            }

            #[allow(unused)]
            fn try_into_vec(self) -> VmResult<Vec<Value>> {
                let ($($value,)*) = self;
                let mut vec = vm_try!(Vec::try_with_capacity($count));
                $(vm_try!(vec.try_push(vm_try!(<$ty>::to_value($value))));)*
                VmResult::Ok(vec)
            }

            fn count(&self) -> usize {
                $count
            }
        }
    };
}

repeat_macro!(impl_into_args);

impl Args for Vec<Value> {
    fn into_stack(self, stack: &mut Stack) -> VmResult<()> {
        for value in self {
            vm_try!(stack.push(value));
        }

        VmResult::Ok(())
    }

    #[inline]
    fn try_into_vec(self) -> VmResult<Vec<Value>> {
        VmResult::Ok(self)
    }

    #[inline]
    fn count(&self) -> usize {
        self.len()
    }
}

#[cfg(feature = "alloc")]
impl Args for ::rust_alloc::vec::Vec<Value> {
    fn into_stack(self, stack: &mut Stack) -> VmResult<()> {
        for value in self {
            vm_try!(stack.push(value));
        }

        VmResult::Ok(())
    }

    #[inline]
    fn try_into_vec(self) -> VmResult<Vec<Value>> {
        VmResult::Ok(vm_try!(Vec::try_from(self)))
    }

    #[inline]
    fn count(&self) -> usize {
        self.len()
    }
}
