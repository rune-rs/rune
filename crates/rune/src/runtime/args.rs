use core::fmt;

use crate::alloc::Vec;
use crate::runtime::{GuardedArgs, Stack, ToValue, Value, VmError};

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
    fn push_to_stack(&mut self, stack: &mut Stack) -> Result<(), VmError>;

    /// Get the number of arguments.
    fn count(&self) -> usize;
}

impl DynArgs for () {
    #[inline]
    fn push_to_stack(&mut self, _: &mut Stack) -> Result<(), VmError> {
        Ok(())
    }

    #[inline]
    fn count(&self) -> usize {
        0
    }
}

impl<T> DynArgs for Option<T>
where
    T: Args,
{
    fn push_to_stack(&mut self, stack: &mut Stack) -> Result<(), VmError> {
        let Some(args) = self.take() else {
            return Err(VmError::new(DynArgsUsed));
        };

        args.into_stack(stack)?;
        Ok(())
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
    fn push_to_stack(&mut self, stack: &mut Stack) -> Result<(), VmError> {
        let Some(value) = self.value.take() else {
            return Err(VmError::new(DynArgsUsed));
        };

        // SAFETY: We've setup the type so that the caller cannot ignore the guard.
        self.guard = unsafe { Some(GuardedArgs::guarded_into_stack(value, stack)?) };
        Ok(())
    }

    fn count(&self) -> usize {
        self.value.as_ref().map_or(0, GuardedArgs::count)
    }
}

/// Trait for converting arguments into an array.
pub trait FixedArgs<const N: usize> {
    /// Encode arguments as array.
    fn into_array(self) -> Result<[Value; N], VmError>;
}

/// Trait for converting arguments onto the stack.
pub trait Args {
    /// Encode arguments onto a stack.
    fn into_stack(self, stack: &mut Stack) -> Result<(), VmError>;

    /// Convert arguments into a vector.
    fn try_into_vec(self) -> Result<Vec<Value>, VmError>;

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
            fn into_array(self) -> Result<[Value; $count], VmError> {
                let ($($value,)*) = self;
                $(let $value = $value.to_value()?;)*
                Ok([$($value),*])
            }
        }

        impl<$($ty,)*> Args for ($($ty,)*)
        where
            $($ty: ToValue,)*
        {
            #[allow(unused)]
            fn into_stack(self, stack: &mut Stack) -> Result<(), VmError> {
                let ($($value,)*) = self;
                $(stack.push($value.to_value()?)?;)*
                Ok(())
            }

            #[allow(unused)]
            fn try_into_vec(self) -> Result<Vec<Value>, VmError> {
                let ($($value,)*) = self;
                let mut vec = Vec::try_with_capacity($count)?;
                $(vec.try_push(<$ty>::to_value($value)?)?;)*
                Ok(vec)
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
    #[inline]
    fn into_stack(self, stack: &mut Stack) -> Result<(), VmError> {
        for value in self {
            stack.push(value)?;
        }

        Ok(())
    }

    #[inline]
    fn try_into_vec(self) -> Result<Vec<Value>, VmError> {
        Ok(self)
    }

    #[inline]
    fn count(&self) -> usize {
        self.len()
    }
}

impl Args for rust_alloc::vec::Vec<Value> {
    #[inline]
    fn into_stack(self, stack: &mut Stack) -> Result<(), VmError> {
        for value in self {
            stack.push(value)?;
        }

        Ok(())
    }

    #[inline]
    fn try_into_vec(self) -> Result<Vec<Value>, VmError> {
        Ok(Vec::try_from(self)?)
    }

    #[inline]
    fn count(&self) -> usize {
        self.len()
    }
}
