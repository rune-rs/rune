use core::future::Future;

use futures_util::never::Never;

use crate::{
    runtime::{self, Stack, ToValue, TypeOf, UnsafeFromValue, VmErrorKind, VmResult},
    Value,
};

macro_rules! check_args {
    ($expected:expr, $actual:expr) => {
        if $actual != $expected {
            return VmResult::err(VmErrorKind::BadArgumentCount {
                actual: $actual,
                expected: $expected,
            });
        }
    };
}

// Expand to instance variable bindings.
macro_rules! unsafe_inst_vars {
    ($inst:ident, $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        let $inst = vm_try!(Instance::from_value($inst).with_error(|| VmErrorKind::BadArgument {
            arg: 0,
        }));

        $(
            let $var = vm_try!(<$ty>::from_value($var).with_error(|| VmErrorKind::BadArgument {
                arg: 1 + $num,
            }));
        )*
    };
}

/// Denotes the kind of a function, allowing the [`Function`] trait to be
/// implemented separately for plain and async functions.
pub trait FunctionKind {
    /// Indicates if the function is async.
    fn is_async() -> bool;
}

pub trait RawFunctionKind<Marker, F>: FunctionKind
where
    F: RawFunction<Marker>,
{
    type Return;
    fn fn_call(ret: F::RawReturn, guard: F::Guard) -> VmResult<Value>;
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
impl<Marker, F> RawFunctionKind<Marker, F> for Plain
where
    F: RawFunction<Marker>,
    F::RawReturn: ToValue,
{
    type Return = F::RawReturn;
    fn fn_call(ret: F::RawReturn, guard: F::Guard) -> VmResult<Value> {
        unsafe {
            <F as RawFunction<Marker>>::drop_guard(guard);
        }
        ret.to_value()
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
impl<Marker, F> RawFunctionKind<Marker, F> for Async
where
    F: RawFunction<Marker>,
    F::Guard: 'static,
    F::RawReturn: 'static + Future,
    <F::RawReturn as Future>::Output: ToValue,
{
    type Return = <F::RawReturn as Future>::Output;
    fn fn_call(fut: F::RawReturn, guard: F::Guard) -> VmResult<Value> {
        // Safety: Future is owned and will only be called within the
        // context of the virtual machine, which will provide
        // exclusive thread-local access to itself while the future is
        // being polled.
        #[allow(unused)]
        let ret = unsafe {
            runtime::Future::new(async move {
                let ret = fut.await;
                <F as RawFunction<Marker>>::drop_guard(guard);
                let ret = vm_try!(ret.to_value());
                VmResult::Ok(ret)
            })
        };

        ret.to_value()
    }
}

pub trait RawFunction<Marker>: 'static + Send + Sync {
    /// An object guarding the lifetime of the arguments.
    type Guard;

    /// A tuple representing the argument type.
    #[doc(hidden)]
    type FirstArg;

    /// A tuple representing the argument type.
    #[doc(hidden)]
    type Arguments;

    /// The raw return type of the function.
    #[doc(hidden)]
    type RawReturn;

    /// Get the number of arguments.
    #[doc(hidden)]
    fn args() -> usize;

    /// Safety: We hold a reference to the stack, so we can
    /// guarantee that it won't be modified.
    ///
    /// The scope is also necessary, since we mutably access `stack`
    /// when we return below.
    #[must_use]
    #[doc(hidden)]
    unsafe fn fn_call_raw(
        &self,
        stack: &mut Stack,
        args: usize,
    ) -> VmResult<(Self::RawReturn, Self::Guard)>;

    /// This can be cleaned up once the arguments are no longer in use.
    #[doc(hidden)]
    unsafe fn drop_guard(guard: Self::Guard);
}

/// Trait used to provide the [function][crate::module::Module::function]
/// function.
pub trait Function<Marker, K>: RawFunction<Marker> {
    /// The return type of the function.
    #[doc(hidden)]
    type Return;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

/// Trait used to provide the [`associated_function`] function.
///
/// [`associated_function`]: crate::module::Module::associated_function
pub trait InstanceFunction<Marker, K>: RawFunction<Marker> {
    /// The return type of the function.
    #[doc(hidden)]
    type Instance: TypeOf;

    /// The return type of the function.
    #[doc(hidden)]
    type Return;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

impl<U, T> RawFunction<fn() -> U> for T
where
    T: 'static + Send + Sync + Fn() -> U,
{
    type FirstArg = Never;
    type Arguments = ();
    type RawReturn = U;
    type Guard = ();
    fn args() -> usize {
        0
    }
    unsafe fn fn_call_raw(&self, stack: &mut Stack, args: usize) -> VmResult<(U, Self::Guard)> {
        check_args!(0, args);
        let [] = vm_try!(stack.drain_vec(0));
        let that = self();
        VmResult::Ok((that, ()))
    }
    unsafe fn drop_guard(_guard: Self::Guard) {}
}

macro_rules! impl_register {
    ($count:expr $(, $ty:ident $var:ident $num:expr)*) => {
        impl<U, T, Instance, $($ty),*> RawFunction<fn(Instance, $($ty,)*) -> U> for T
        where
            T: 'static + Send + Sync + Fn(Instance, $($ty,)*) -> U,
            Instance: UnsafeFromValue,
            $($ty: UnsafeFromValue,)*
        {
            type FirstArg = Instance;
            type Arguments = (Instance, $($ty,)*);
            type RawReturn = U;
            type Guard = (Instance::Guard, $($ty::Guard,)*);
            fn args() -> usize {
                $count + 1
            }
            unsafe fn fn_call_raw(&self, stack: &mut Stack, args: usize) -> VmResult<(U, Self::Guard)> {
                check_args!($count+1, args);
                let [inst $(, $var)*] = vm_try!(stack.drain_vec($count+1));

                unsafe_inst_vars!(inst, $count, $($ty, $var, $num,)*);
                let that = self(Instance::unsafe_coerce(inst.0), $(<$ty>::unsafe_coerce($var.0),)*);
                VmResult::Ok((that, (inst.1, $($var.1,)*)))
            }
            unsafe fn drop_guard(guard: Self::Guard) {
                let (inst, $($var,)*) = guard;
                drop(inst);
                $(drop(($var));)*
            }
        }
    };
}

impl<T, Marker> Function<Marker, Plain> for T
where
    T: RawFunction<Marker>,
    Plain: RawFunctionKind<Marker, T>,
{
    type Return = <Plain as RawFunctionKind<Marker, T>>::Return;

    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
        let (ret, guard) =
            vm_try!(unsafe { <T as RawFunction<Marker>>::fn_call_raw(self, stack, args) });

        let ret = vm_try!(Plain::fn_call(ret, guard));

        stack.push(ret);
        VmResult::Ok(())
    }
}

impl<'a, T, Marker> Function<Marker, Async> for T
where
    T: RawFunction<Marker>,
    Async: RawFunctionKind<Marker, T>,
{
    type Return = <Async as RawFunctionKind<Marker, T>>::Return;

    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
        let (ret, guard) =
            vm_try!(unsafe { <T as RawFunction<Marker>>::fn_call_raw(self, stack, args) });

        let ret = vm_try!(Async::fn_call(ret, guard));

        stack.push(ret);
        VmResult::Ok(())
    }
}

impl<'a, T, Marker> InstanceFunction<Marker, Plain> for T
where
    T: RawFunction<Marker>,
    Plain: RawFunctionKind<Marker, T>,
    <T as RawFunction<Marker>>::FirstArg: TypeOf,
{
    type Instance = <T as RawFunction<Marker>>::FirstArg;
    type Return = <Plain as RawFunctionKind<Marker, T>>::Return;

    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
        let (ret, guard) =
            vm_try!(unsafe { <T as RawFunction<Marker>>::fn_call_raw(self, stack, args) });

        let ret = vm_try!(Plain::fn_call(ret, guard));

        stack.push(ret);
        VmResult::Ok(())
    }
}

impl<'a, T, Marker> InstanceFunction<Marker, Async> for T
where
    T: RawFunction<Marker>,
    Async: RawFunctionKind<Marker, T>,
    <T as RawFunction<Marker>>::FirstArg: TypeOf,
{
    type Instance = <T as RawFunction<Marker>>::FirstArg;
    type Return = <Async as RawFunctionKind<Marker, T>>::Return;

    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
        let (ret, guard) =
            vm_try!(unsafe { <T as RawFunction<Marker>>::fn_call_raw(self, stack, args) });

        let ret = vm_try!(Async::fn_call(ret, guard));

        stack.push(ret);
        VmResult::Ok(())
    }
}

repeat_macro!(impl_register);
