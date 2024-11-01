use core::marker::PhantomData;

use rust_alloc::sync::Arc;

use crate::runtime::{FixedArgs, FunctionHandler, InstAddress, Output, VmResult};
use crate::FromValue;

/// Helper struct to conveniently call native functions.
///
/// Note: This can only be used with functions that take at least one argument.
/// Otherwise it will panic.
#[derive(Clone)]
pub(crate) struct Caller<A, const N: usize, T> {
    handler: Arc<FunctionHandler>,
    _marker: PhantomData<(A, T)>,
}

impl<A, const N: usize, T> Caller<A, N, T>
where
    A: FixedArgs<N>,
    T: FromValue,
{
    /// Construct a new caller helper
    pub(crate) fn new(handler: Arc<FunctionHandler>) -> Self {
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
    pub(crate) fn call(&self, args: A) -> VmResult<T> {
        const {
            assert!(N > 0, "Must be used with non-zero arguments");
        }

        let mut args = vm_try!(args.into_array());

        vm_try!((self.handler)(
            &mut args,
            InstAddress::ZERO,
            N,
            Output::keep(0)
        ));

        let Some(value) = args.into_iter().next() else {
            unreachable!();
        };

        VmResult::Ok(vm_try!(T::from_value(value)))
    }
}

// SAFETY: The marker doesn't matter.
unsafe impl<A, const N: usize, T> Send for Caller<A, N, T> {}
unsafe impl<A, const N: usize, T> Sync for Caller<A, N, T> {}
