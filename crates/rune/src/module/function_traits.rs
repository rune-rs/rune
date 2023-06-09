use core::future::Future;

use crate::runtime::{self, Stack, ToValue, TypeOf, UnsafeFromValue, VmErrorKind, VmResult};

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

// Expand to function variable bindings.
macro_rules! unsafe_vars {
    ($count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        $(
            let $var = vm_try!(<$ty>::from_value($var).with_error(|| VmErrorKind::BadArgument {
                arg: $num,
            }));
        )*
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

pub trait FunctionPrefix<Marker>: 'static + Send + Sync {
    /// An object guarding the lifetime of the arguments.
    type Guard;

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
pub trait Function<Marker, K>: FunctionPrefix<Marker> {
    /// The return type of the function.
    #[doc(hidden)]
    type Return;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

pub trait InstanceFunctionPrefix<Marker>: 'static + Send + Sync {
    /// An object guarding the lifetime of the arguments.
    type Guard;

    /// The type of the instance.
    #[doc(hidden)]
    type Instance: TypeOf;

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

/// Trait used to provide the [`associated_function`] function.
///
/// [`associated_function`]: crate::module::Module::associated_function
pub trait InstanceFunction<Marker, K>: InstanceFunctionPrefix<Marker> {
    /// The return type of the function.
    #[doc(hidden)]
    type Return;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

macro_rules! impl_register {
    ($count:expr $(, $ty:ident $var:ident $num:expr)*) => {
        impl<U, T, $($ty),*> FunctionPrefix<fn($($ty,)*) -> U> for T
        where
            T: 'static + Send + Sync + Fn($($ty,)*) -> U,
            $($ty: UnsafeFromValue,)*
        {
            type Arguments = ($($ty,)*);
            type RawReturn = U;
            type Guard = ($($ty::Guard,)*);
            fn args() -> usize {
                $count
            }
            unsafe fn fn_call_raw(&self, stack: &mut Stack, args: usize) -> VmResult<(U, Self::Guard)> {
                check_args!($count, args);
                let [$($var,)*] = vm_try!(stack.drain_vec($count));

                unsafe_vars!($count, $($ty, $var, $num,)*);
                let that = self($(<$ty>::unsafe_coerce($var.0),)*);
                VmResult::Ok((that, ($($var.1,)*)))
            }
            unsafe fn drop_guard(guard: Self::Guard) {
                let ($($var,)*) = guard;
                $(drop(($var));)*
            }
        }

        impl<U, T, Instance, $($ty),*> InstanceFunctionPrefix<fn(Instance, $($ty,)*) -> U> for T
        where
            T: 'static + Send + Sync + Fn(Instance, $($ty,)*) -> U,
            Instance: UnsafeFromValue + TypeOf,
            $($ty: UnsafeFromValue,)*
        {
            type Instance = Instance;
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

impl<'a, T, Marker> Function<Marker, Plain> for T
where
    T: FunctionPrefix<Marker>,
    <T as FunctionPrefix<Marker>>::RawReturn: ToValue,
{
    type Return = <T as FunctionPrefix<Marker>>::RawReturn;

    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
        let (ret, guard) =
            vm_try!(unsafe { <T as FunctionPrefix<Marker>>::fn_call_raw(self, stack, args) });

        unsafe {
            <T as FunctionPrefix<Marker>>::drop_guard(guard);
        }

        let ret = vm_try!(ret.to_value());
        stack.push(ret);
        VmResult::Ok(())
    }
}

impl<'a, T, Marker> Function<Marker, Async> for T
where
    T: FunctionPrefix<Marker>,
    <T as FunctionPrefix<Marker>>::Guard: 'static,
    <T as FunctionPrefix<Marker>>::RawReturn: 'static + Future,
    <<T as FunctionPrefix<Marker>>::RawReturn as Future>::Output: ToValue,
{
    type Return = <<T as FunctionPrefix<Marker>>::RawReturn as Future>::Output;

    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
        let (fut, guard) =
            vm_try!(unsafe { <T as FunctionPrefix<Marker>>::fn_call_raw(self, stack, args) });

        // Safety: Future is owned and will only be called within the
        // context of the virtual machine, which will provide
        // exclusive thread-local access to itself while the future is
        // being polled.
        #[allow(unused)]
        let ret = unsafe {
            runtime::Future::new(async move {
                let ret = fut.await;
                <T as FunctionPrefix<Marker>>::drop_guard(guard);
                let ret = vm_try!(ret.to_value());
                VmResult::Ok(ret)
            })
        };

        let ret = vm_try!(ret.to_value());
        stack.push(ret);
        VmResult::Ok(())
    }
}

impl<'a, T, Marker> InstanceFunction<Marker, Plain> for T
where
    T: InstanceFunctionPrefix<Marker>,
    <T as InstanceFunctionPrefix<Marker>>::RawReturn: ToValue,
{
    type Return = <T as InstanceFunctionPrefix<Marker>>::RawReturn;

    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
        let (ret, guard) = vm_try!(unsafe {
            <T as InstanceFunctionPrefix<Marker>>::fn_call_raw(self, stack, args)
        });

        unsafe {
            <T as InstanceFunctionPrefix<Marker>>::drop_guard(guard);
        }

        let ret = vm_try!(ret.to_value());
        stack.push(ret);
        VmResult::Ok(())
    }
}

impl<'a, T, Marker> InstanceFunction<Marker, Async> for T
where
    T: InstanceFunctionPrefix<Marker>,
    <T as InstanceFunctionPrefix<Marker>>::Guard: 'static,
    <T as InstanceFunctionPrefix<Marker>>::RawReturn: 'static + Future,
    <<T as InstanceFunctionPrefix<Marker>>::RawReturn as Future>::Output: ToValue,
{
    type Return = <<T as InstanceFunctionPrefix<Marker>>::RawReturn as Future>::Output;

    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
        let (fut, guard) = vm_try!(unsafe {
            <T as InstanceFunctionPrefix<Marker>>::fn_call_raw(self, stack, args)
        });

        // Safety: Future is owned and will only be called within the
        // context of the virtual machine, which will provide
        // exclusive thread-local access to itself while the future is
        // being polled.
        #[allow(unused)]
        let ret = unsafe {
            runtime::Future::new(async move {
                let ret = fut.await;
                <T as InstanceFunctionPrefix<Marker>>::drop_guard(guard);
                let ret = vm_try!(ret.to_value());
                VmResult::Ok(ret)
            })
        };

        let ret = vm_try!(ret.to_value());
        stack.push(ret);
        VmResult::Ok(())
    }
}

repeat_macro!(impl_register);
