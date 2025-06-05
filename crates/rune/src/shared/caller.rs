use core::marker::PhantomData;

use crate::runtime::{Address, FixedArgs, FunctionHandler, Output, VmError};
use crate::FromValue;

/// Helper struct to conveniently call native functions.
///
/// Note: This can only be used with functions that take at least one argument.
/// Otherwise it will panic.
#[derive(Clone)]
pub(crate) struct Caller<A, const N: usize, T> {
    handler: FunctionHandler,
    _marker: PhantomData<(A, T)>,
}

impl<A, const N: usize, T> Caller<A, N, T>
where
    A: FixedArgs<N>,
    T: FromValue,
{
    /// Construct a new caller helper
    pub(crate) fn new(handler: FunctionHandler) -> Self {
        Self {
            handler,
            _marker: PhantomData,
        }
    }

    /// Modify the return value of the caller.
    pub(crate) fn with_return<U>(&self) -> Caller<A, N, U>
    where
        U: FromValue,
    {
        Caller {
            handler: self.handler.clone(),
            _marker: PhantomData,
        }
    }

    /// Perform a call.
    pub(crate) fn call(&self, args: A) -> Result<T, VmError> {
        const {
            assert!(N > 0, "Must be used with non-zero arguments");
        }

        let mut args = args.into_array()?;

        self.handler
            .call(&mut args, Address::ZERO, N, Output::keep(0))?;

        let Some(value) = args.into_iter().next() else {
            unreachable!();
        };

        Ok(T::from_value(value)?)
    }
}

// SAFETY: The marker doesn't matter.
unsafe impl<A, const N: usize, T> Send for Caller<A, N, T> {}
unsafe impl<A, const N: usize, T> Sync for Caller<A, N, T> {}
