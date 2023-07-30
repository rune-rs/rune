use core::future::Future;

use crate::runtime::{self, Stack, ToValue, TypeOf, UnsafeFromValue, VmErrorKind, VmResult};

// Expand to function variable bindings.
macro_rules! drain_stack {
    ($count:expr, $add:expr, $stack:ident, $args:ident, $($ty:ty, $var:ident, $num:expr),* $(,)?) => {
        if $args != $count + $add {
            return VmResult::err(VmErrorKind::BadArgumentCount {
                actual: $args,
                expected: $count + $add,
            });
        }

        let [$($var,)*] = vm_try!($stack.drain_vec($count + $add));

        $(
            let $var = vm_try!(<$ty>::unsafe_from_value($var).with_error(|| VmErrorKind::BadArgument {
                arg: $num,
            }));
        )*
    };
}

macro_rules! unsafe_coerce {
    ($target:ident $(, $ty:ty, $var:expr)*) => {
        $target($(unsafe { <$ty>::unsafe_coerce($var) },)*)
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

/// Trait used to provide the [function][crate::module::Module::function]
/// function.
pub trait Function<A, K>: 'static + Send + Sync {
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
pub trait InstanceFunction<A, K>: 'static + Send + Sync {
    /// The type of the instance.
    #[doc(hidden)]
    type Instance: TypeOf;

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

macro_rules! impl_register {
    ($count:expr $(, $ty:ident $var:ident $num:expr)*) => {
        impl<T, U, $($ty,)*> Function<($($ty,)*), Plain> for T
        where
            T: 'static + Send + Sync + Fn($($ty,)*) -> U,
            U: ToValue,
            $($ty: UnsafeFromValue,)*
        {
            type Return = U;

            fn args() -> usize {
                $count
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                drain_stack!($count, 0, stack, args, $($ty, $var, $num,)*);

                // Safety: We hold a reference to the stack, so we can guarantee
                // that it won't be modified.
                let ret = unsafe_coerce!(self $(, $ty, $var.0)*);
                $(drop($var.1);)*

                let ret = vm_try!(ret.to_value());
                stack.push(ret);
                VmResult::Ok(())
            }
        }

        impl<T, U, $($ty,)*> Function<($($ty,)*), Async> for T
        where
            T: 'static + Send + Sync + Fn($($ty,)*) -> U,
            U: 'static + Future,
            U::Output: ToValue,
            $($ty: 'static + UnsafeFromValue,)*
        {
            type Return = U::Output;

            fn args() -> usize {
                $count
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                drain_stack!($count, 0, stack, args, $($ty, $var, $num,)*);

                // Safety: The future holds onto all necessary guards to keep
                // values borrowed from the stack alive.
                let fut = unsafe_coerce!(self $(, $ty, $var.0)*);

                let ret = runtime::Future::new(async move {
                    let output = fut.await;
                    $(drop($var.1);)*
                    VmResult::Ok(vm_try!(output.to_value()))
                });

                stack.push(ret);
                VmResult::Ok(())
            }
        }

        impl<T, U, Instance, $($ty,)*> InstanceFunction<(Instance, $($ty,)*), Plain> for T
        where
            T: 'static + Send + Sync + Fn(Instance $(, $ty)*) -> U,
            U: ToValue,
            Instance: UnsafeFromValue + TypeOf,
            $($ty: UnsafeFromValue,)*
        {
            type Instance = Instance;
            type Return = U;

            #[inline]
            fn args() -> usize {
                $count + 1
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                drain_stack!($count, 1, stack, args, Instance, inst, 0 $(, $ty, $var, 1 + $num)*);

                // Safety: We hold a reference to the stack, so we can
                // guarantee that it won't be modified.
                let ret = unsafe_coerce!(self, Instance, inst.0 $(, $ty, $var.0)*);

                drop(inst.1);
                $(drop($var.1);)*

                stack.push(vm_try!(ret.to_value()));
                VmResult::Ok(())
            }
        }

        impl<T, U, Instance, $($ty,)*> InstanceFunction<(Instance, $($ty,)*), Async> for T
        where
            T: 'static + Send + Sync + Fn(Instance $(, $ty)*) -> U,
            U: 'static + Future,
            U::Output: ToValue,
            Instance: UnsafeFromValue + TypeOf,
            $($ty: UnsafeFromValue,)*
        {
            type Instance = Instance;
            type Return = U::Output;

            #[inline]
            fn args() -> usize {
                $count + 1
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                drain_stack!($count, 1, stack, args, Instance, inst, 0 $(, $ty, $var, 1 + $num)*);

                // Safety: The future holds onto all necessary guards to keep
                // values borrowed from the stack alive.
                let fut = unsafe_coerce!(self, Instance, inst.0 $(, $ty, $var.0)*);

                let ret = runtime::Future::new(async move {
                    let output = fut.await;
                    drop(inst.1);
                    $(drop($var.1);)*
                    VmResult::Ok(vm_try!(output.to_value()))
                });

                stack.push(ret);
                VmResult::Ok(())
            }
        }
    };
}

repeat_macro!(impl_register);
