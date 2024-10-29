use core::marker::PhantomData;

use rust_alloc::sync::Arc;

use crate::runtime::{FixedArgs, FunctionHandler, InstAddress, Output, VmResult};
use crate::FromValue;

/// Helper struct to conveniently call native functions.
///
/// Note: This can only be used with functions that take at least one argument.
/// Otherwise it will panic.
#[derive(Clone)]
pub(crate) struct Caller<T> {
    handler: Arc<FunctionHandler>,
    _marker: PhantomData<T>,
}

impl<T> Caller<T>
where
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
    pub(crate) fn with_return<U>(&self) -> Caller<U>
    where
        U: FromValue,
    {
        Caller {
            handler: self.handler.clone(),
            _marker: PhantomData,
        }
    }

    /// Perform a call.
    pub(crate) fn call<const N: usize>(&self, args: impl FixedArgs<N>) -> VmResult<T> {
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
unsafe impl<T> Send for Caller<T> {}
unsafe impl<T> Sync for Caller<T> {}
