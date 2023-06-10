use futures_core::Future;

use crate::{
    runtime::{self, Stack, TypeOf, VmResult},
    ToValue,
};

use super::function_raw_traits::RawFunction;

/// Denotes the kind of a function, allowing the [`Function`] trait to be
/// implemented separately for plain and async functions.
pub trait FunctionKind {
    /// Indicates if the function is async.
    fn is_async() -> bool;
}

/// Marker for plain functions.
#[non_exhaustive]
pub struct Plain;

impl FunctionKind for Plain {
    #[inline]
    fn is_async() -> bool {
        false
    }
}

/// Marker for async functions.
#[non_exhaustive]
pub struct Async;

impl FunctionKind for Async {
    #[inline]
    fn is_async() -> bool {
        true
    }
}

/// Trait used to provide the [function][crate::module::Module::function]
/// function.
pub trait Function<Marker, K>: 'static + Send + Sync {
    /// The arguments of the function.
    #[doc(hidden)]
    type Arguments;

    /// The return type of the function.
    #[doc(hidden)]
    type Return;

    /// Get the number of arguments.
    #[doc(hidden)]
    fn args() -> usize;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

/// Trait used to provide the [`associated_function`] function.
///
/// [`associated_function`]: crate::module::Module::associated_function
pub trait InstanceFunction<Marker, K>: 'static + Send + Sync {
    /// The type of the instance.
    #[doc(hidden)]
    type Instance: TypeOf;

    /// The arguments of the function.
    #[doc(hidden)]
    type Arguments;

    /// The return type of the function.
    #[doc(hidden)]
    type Return;

    /// Get the number of arguments.
    #[doc(hidden)]
    fn args() -> usize;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

impl<T, Marker> Function<Marker, Plain> for T
where
    T: RawFunction<Marker>,
    <T as RawFunction<Marker>>::RawReturn: ToValue,
{
    type Arguments = T::RawArguments;
    type Return = <T as RawFunction<Marker>>::RawReturn;

    fn args() -> usize {
        <T as RawFunction<Marker>>::raw_args()
    }

    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
        let packet = vm_try!(unsafe { T::raw_get_args(self, stack, args) });
        let (ret, guard) = vm_try!(unsafe { T::raw_call_packet(self, packet) });
        unsafe { T::raw_drop_guard(guard) };
        let ret = vm_try!(ret.to_value());
        stack.push(ret);
        VmResult::Ok(())
    }
}

impl<'a, T, Marker> Function<Marker, Async> for T
where
    T: RawFunction<Marker>,
    <T as RawFunction<Marker>>::RawGuard: 'static,
    <T as RawFunction<Marker>>::RawReturn: 'static + Future,
    <<T as RawFunction<Marker>>::RawReturn as Future>::Output: ToValue,
{
    type Arguments = T::RawArguments;
    type Return = <<T as RawFunction<Marker>>::RawReturn as Future>::Output;

    fn args() -> usize {
        <T as RawFunction<Marker>>::raw_args()
    }

    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
        let packet = vm_try!(unsafe { T::raw_get_args(self, stack, args) });
        let (fut, guard) = vm_try!(unsafe { T::raw_call_packet(self, packet) });

        #[allow(unused)]
        let ret = runtime::Future::new(async move {
            let ret = fut.await;
            // Safety: We no longer need the stack to not be modified at this point.
            unsafe { <T as RawFunction<Marker>>::raw_drop_guard(guard) };
            let ret = vm_try!(ret.to_value());
            VmResult::Ok(ret)
        });

        let ret = vm_try!(ret.to_value());
        stack.push(ret);
        VmResult::Ok(())
    }
}

impl<'a, T, K, Marker> InstanceFunction<Marker, K> for T
where
    T: RawFunction<Marker> + Function<Marker, K>,
    <T as RawFunction<Marker>>::RawArgumentFirst: TypeOf,
{
    type Instance = <T as RawFunction<Marker>>::RawArgumentFirst;
    type Arguments = T::Arguments;
    type Return = <T as Function<Marker, K>>::Return;

    fn args() -> usize {
        <T as Function<Marker, K>>::args()
    }

    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
        <T as Function<Marker, K>>::fn_call(&self, stack, args)
    }
}
