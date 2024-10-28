use core::fmt;

use crate::alloc::Vec;
use crate::runtime::{GuardedArgs, Stack, ToValue, Value, VmResult};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) struct DynArgsUsed;

impl fmt::Display for DynArgsUsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dynamic arguments have already been used")
    }
}

/// Object safe variant of args which errors instead of consumed itself.
pub(crate) trait DynArgs {
    /// Encode arguments onto a stack.
    fn push_to_stack(&mut self, stack: &mut Stack) -> VmResult<()>;

    /// Get the number of arguments.
    fn count(&self) -> usize;
}

impl DynArgs for () {
    fn push_to_stack(&mut self, _: &mut Stack) -> VmResult<()> {
        VmResult::Ok(())
    }

    fn count(&self) -> usize {
        0
    }
}

impl<T> DynArgs for Option<T>
where
    T: Args,
{
    fn push_to_stack(&mut self, stack: &mut Stack) -> VmResult<()> {
        let Some(args) = self.take() else {
            return VmResult::err(DynArgsUsed);
        };

        vm_try!(args.into_stack(stack));
        VmResult::Ok(())
    }

    fn count(&self) -> usize {
        self.as_ref().map_or(0, Args::count)
    }
}

pub(crate) struct DynGuardedArgs<T>
where
    T: GuardedArgs,
{
    value: Option<T>,
    guard: Option<T::Guard>,
}

impl<T> DynGuardedArgs<T>
where
    T: GuardedArgs,
{
    pub(crate) fn new(value: T) -> Self {
        Self {
            value: Some(value),
            guard: None,
        }
    }
}

impl<T> DynArgs for DynGuardedArgs<T>
where
    T: GuardedArgs,
{
    fn push_to_stack(&mut self, stack: &mut Stack) -> VmResult<()> {
        let Some(value) = self.value.take() else {
            return VmResult::err(DynArgsUsed);
        };

        // SAFETY: We've setup the type so that the caller cannot ignore the guard.
        self.guard = unsafe { Some(vm_try!(GuardedArgs::unsafe_into_stack(value, stack))) };

        VmResult::Ok(())
    }

    fn count(&self) -> usize {
        self.value.as_ref().map_or(0, GuardedArgs::count)
    }
}

/// Trait for converting arguments into an array.
pub trait FixedArgs<const N: usize> {
    /// Encode arguments as array.
    fn into_array(self) -> VmResult<[Value; N]>;
}

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
        impl<$($ty,)*> FixedArgs<$count> for ($($ty,)*)
        where
            $($ty: ToValue,)*
        {
            #[allow(unused)]
            fn into_array(self) -> VmResult<[Value; $count]> {
                let ($($value,)*) = self;
                $(let $value = vm_try!($value.to_value());)*
                VmResult::Ok([$($value),*])
            }
        }

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

            #[inline]
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
