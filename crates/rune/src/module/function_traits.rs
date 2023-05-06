use core::future::Future;

use crate::runtime::{
    self, Stack, ToValue, TypeInfo, TypeOf, UnsafeFromValue, VmErrorKind, VmResult,
};
use crate::Hash;

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

// Helper variation to drop all stack guards associated with the specified variables.
macro_rules! drop_stack_guards {
    ($($var:ident),* $(,)?) => {{
        $(drop(($var.1));)*
    }};
}

// Expand to instance variable bindings.
macro_rules! unsafe_inst_vars {
    ($inst:ident, $count:expr, $($ty:ty, $var:ident, $num:expr,)*) => {
        let $inst = vm_try!(Inst::from_value($inst).with_error(|| VmErrorKind::BadArgument {
            arg: 0,
        }));

        $(
            let $var = vm_try!(<$ty>::from_value($var).with_error(|| VmErrorKind::BadArgument {
                arg: 1 + $num,
            }));
        )*
    };
}

/// The static hash and diagnostical information about a type.
#[derive(Debug, Clone)]
#[non_exhaustive]
#[doc(hidden)]
pub struct AssocType {
    /// Hash of the type.
    pub hash: Hash,
    /// Type information of the instance function.
    pub type_info: TypeInfo,
}

/// Trait used to provide the [function][crate::module::Module::function]
/// function.
pub trait Function<A>: 'static + Send + Sync {
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

/// Trait used to provide the
/// [async_function][crate::module::Module::async_function] function.
pub trait AsyncFunction<A>: 'static + Send + Sync {
    /// The return type of the function.
    #[doc(hidden)]
    type Return: Future<Output = Self::Output>;

    /// The output produces by the future.
    #[doc(hidden)]
    type Output;

    /// Get the number of arguments.
    #[doc(hidden)]
    fn args() -> usize;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

/// Trait used to provide the [inst_fn][crate::module::Module::inst_fn]
/// function.
pub trait InstFn<A>: 'static + Send + Sync {
    /// The type of the instance.
    #[doc(hidden)]
    type Inst;

    /// The return type of the function.
    #[doc(hidden)]
    type Return;

    /// Get the number of arguments.
    #[doc(hidden)]
    fn args() -> usize;

    /// Access static information on the instance type with the associated
    /// function.
    #[doc(hidden)]
    fn ty() -> AssocType;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

/// Trait used to provide the
/// [async_inst_fn][crate::module::Module::async_inst_fn] function.
pub trait AsyncInstFn<A>: 'static + Send + Sync {
    /// The type of the instance.
    #[doc(hidden)]
    type Inst;

    /// The return type of the function.
    #[doc(hidden)]
    type Return: Future<Output = Self::Output>;

    /// The output value of the async function.
    #[doc(hidden)]
    type Output;

    /// Get the number of arguments.
    #[doc(hidden)]
    fn args() -> usize;

    /// Access static information on the instance type with the associated
    /// function.
    #[doc(hidden)]
    fn ty() -> AssocType;

    /// Perform the vm call.
    #[doc(hidden)]
    fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()>;
}

macro_rules! impl_register {
    ($count:expr $(, $ty:ident $var:ident $num:expr)*) => {
        impl<T, U, $($ty,)*> Function<($($ty,)*)> for T
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
                check_args!($count, args);
                let [$($var,)*] = vm_try!(stack.drain_vec($count));

                // Safety: We hold a reference to the stack, so we can
                // guarantee that it won't be modified.
                //
                // The scope is also necessary, since we mutably access `stack`
                // when we return below.
                #[allow(unused)]
                let ret = unsafe {
                    unsafe_vars!($count, $($ty, $var, $num,)*);
                    let ret = self($(<$ty>::unsafe_coerce($var.0),)*);
                    drop_stack_guards!($($var),*);
                    ret
                };

                let ret = vm_try!(ret.to_value());
                stack.push(ret);
                VmResult::Ok(())
            }
        }

        impl<T, U, $($ty,)*> AsyncFunction<($($ty,)*)> for T
        where
            T: 'static + Send + Sync + Fn($($ty,)*) -> U,
            U: 'static + Future,
            U::Output: ToValue,
            $($ty: 'static + UnsafeFromValue,)*
        {
            type Return = U;
            type Output = U::Output;

            fn args() -> usize {
                $count
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                check_args!($count, args);
                let [$($var,)*] = vm_try!(stack.drain_vec($count));

                // Safety: Future is owned and will only be called within the
                // context of the virtual machine, which will provide
                // exclusive thread-local access to itself while the future is
                // being polled.
                #[allow(unused_unsafe)]
                let ret = unsafe {
                    unsafe_vars!($count, $($ty, $var, $num,)*);
                    let fut = self($(<$ty>::unsafe_coerce($var.0),)*);

                    runtime::Future::new(async move {
                        let output = fut.await;
                        drop_stack_guards!($($var),*);
                        let value = vm_try!(output.to_value());
                        VmResult::Ok(value)
                    })
                };

                let ret = vm_try!(ret.to_value());
                stack.push(ret);
                VmResult::Ok(())
            }
        }

        impl<T, U, Inst, $($ty,)*> InstFn<(Inst, $($ty,)*)> for T
        where
            T: 'static + Send + Sync + Fn(Inst $(, $ty)*) -> U,
            U: ToValue,
            Inst: UnsafeFromValue + TypeOf,
            $($ty: UnsafeFromValue,)*
        {
            type Inst = Inst;
            type Return = U;

            fn args() -> usize {
                $count + 1
            }

            fn ty() -> AssocType {
                AssocType {
                    hash: Inst::type_hash(),
                    type_info: Inst::type_info(),
                }
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                check_args!(($count + 1), args);
                let [inst $(, $var)*] = vm_try!(stack.drain_vec($count + 1));

                // Safety: We hold a reference to the stack, so we can
                // guarantee that it won't be modified.
                //
                // The scope is also necessary, since we mutably access `stack`
                // when we return below.
                #[allow(unused)]
                let ret = unsafe {
                    unsafe_inst_vars!(inst, $count, $($ty, $var, $num,)*);
                    let ret = self(Inst::unsafe_coerce(inst.0), $(<$ty>::unsafe_coerce($var.0),)*);
                    drop_stack_guards!(inst, $($var),*);
                    ret
                };

                let ret = vm_try!(ret.to_value());
                stack.push(ret);
                VmResult::Ok(())
            }
        }

        impl<T, U, Inst, $($ty,)*> AsyncInstFn<(Inst, $($ty,)*)> for T
        where
            T: 'static + Send + Sync + Fn(Inst $(, $ty)*) -> U,
            U: 'static + Future,
            U::Output: ToValue,
            Inst: UnsafeFromValue + TypeOf,
            $($ty: UnsafeFromValue,)*
        {
            type Inst = Inst;
            type Return = U;
            type Output = U::Output;

            fn args() -> usize {
                $count + 1
            }

            fn ty() -> AssocType {
                AssocType {
                    hash: Inst::type_hash(),
                    type_info: Inst::type_info(),
                }
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                check_args!(($count + 1), args);
                let [inst $(, $var)*] = vm_try!(stack.drain_vec($count + 1));

                // Safety: Future is owned and will only be called within the
                // context of the virtual machine, which will provide
                // exclusive thread-local access to itself while the future is
                // being polled.
                #[allow(unused)]
                let ret = unsafe {
                    unsafe_inst_vars!(inst, $count, $($ty, $var, $num,)*);
                    let fut = self(Inst::unsafe_coerce(inst.0), $(<$ty>::unsafe_coerce($var.0),)*);

                    runtime::Future::new(async move {
                        let output = fut.await;
                        drop_stack_guards!(inst, $($var),*);
                        let value = vm_try!(output.to_value());
                        VmResult::Ok(value)
                    })
                };

                let ret = vm_try!(ret.to_value());
                stack.push(ret);
                VmResult::Ok(())
            }
        }
    };
}

repeat_macro!(impl_register);
