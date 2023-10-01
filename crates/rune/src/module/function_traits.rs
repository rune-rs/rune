#[macro_use]
mod macros;

use core::future::Future;

use crate::hash::Hash;
use crate::runtime::{
    self, FromValue, FullTypeOf, MaybeTypeOf, Stack, ToValue, TypeInfo, TypeOf, UnsafeToMut,
    UnsafeToRef, Value, VmErrorKind, VmResult,
};

// Expand to function variable bindings.
macro_rules! drain_stack {
    ($count:expr, $add:expr, $stack:ident, $args:ident, $($from_fn:path, $var:ident, $num:expr),* $(,)?) => {
        if $args != $count + $add {
            return VmResult::err(VmErrorKind::BadArgumentCount {
                actual: $args,
                expected: $count + $add,
            });
        }

        let [$($var,)*] = vm_try!($stack.drain_vec($count + $add));

        $(
            let $var = vm_try!($from_fn($var).with_error(|| VmErrorKind::BadArgument {
                arg: $num,
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

macro_rules! impl_instance_function_traits {
    ($count:expr $(, $ty:ident $var:ident $num:expr)*) => {
        impl<T, Instance, Kind, $($ty,)*> InstanceFunction<(Instance, $($ty,)*), Kind> for T
        where
            Instance: TypeOf,
            T: Function<(Instance, $($ty,)*), Kind>,
        {
            type Instance = Instance;
            type Return = T::Return;

            #[inline]
            fn args() -> usize {
                <T as Function<(Instance, $($ty,)*), Kind>>::args()
            }

            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                Function::fn_call(self, stack, args)
            }
        }
    };
}

use core::marker::PhantomData;

/// Zero-sized marker struct for references.
pub struct Ref<T: ?Sized>(PhantomData<T>);

impl<T> MaybeTypeOf for Ref<T>
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        T::maybe_type_of()
    }
}

impl<T> TypeOf for Ref<T>
where
    T: ?Sized + TypeOf,
{
    #[inline]
    fn type_of() -> FullTypeOf {
        T::type_of()
    }

    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

/// Zero-sized marker struct for mutable references.
pub struct Mut<T: ?Sized>(PhantomData<T>);

impl<T> MaybeTypeOf for Mut<T>
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        T::maybe_type_of()
    }
}

impl<T> TypeOf for Mut<T>
where
    T: ?Sized + TypeOf,
{
    #[inline]
    fn type_of() -> FullTypeOf {
        T::type_of()
    }

    #[inline]
    fn type_parameters() -> Hash {
        T::type_parameters()
    }

    #[inline]
    fn type_hash() -> Hash {
        T::type_hash()
    }

    #[inline]
    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

// Fake guard for owned values.
struct Guard;

fn from_value<T>(value: Value) -> VmResult<(T, Guard)>
where
    T: FromValue,
{
    VmResult::Ok((vm_try!(T::from_value(value)), Guard))
}

fn unsafe_to_ref<'a, T: ?Sized>(value: Value) -> VmResult<(&'a T, T::Guard)>
where
    T: UnsafeToRef,
{
    // SAFETY: these are only locally used in this module, and we ensure that
    // the guard requirement is met.
    unsafe { T::unsafe_to_ref(value) }
}

fn unsafe_to_mut<'a, T: ?Sized>(value: Value) -> VmResult<(&'a mut T, T::Guard)>
where
    T: UnsafeToMut,
{
    // SAFETY: these are only locally used in this module, and we ensure that
    // the guard requirement is met.
    unsafe { T::unsafe_to_mut(value) }
}

macro_rules! impl_function_traits {
    ($count:expr $(, {$ty:ident, $var:ident, $place:ty, $num:expr, {$($mut:tt)*}, {$($trait:tt)*}, $from_fn:path})*) => {
        impl<T, U, $($ty,)*> Function<($($place,)*), Plain> for T
        where
            T: 'static + Send + Sync + Fn($($($mut)* $ty),*) -> U,
            U: ToValue,
            $($ty: $($trait)*,)*
        {
            type Return = U;

            fn args() -> usize {
                $count
            }

            #[allow(clippy::drop_non_drop)]
            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                drain_stack!($count, 0, stack, args, $($from_fn, $var, $num,)*);

                // Safety: We hold a reference to the stack, so we can guarantee
                // that it won't be modified.
                let ret = self($($var.0),*);
                $(drop($var.1);)*

                let ret = vm_try!(ToValue::to_value(ret));
                vm_try!(stack.push(ret));
                VmResult::Ok(())
            }
        }

        impl<T, U, $($ty,)*> Function<($($place,)*), Async> for T
        where
            T: 'static + Send + Sync + Fn($($($mut)* $ty),*) -> U,
            U: 'static + Future,
            U::Output: ToValue,
            $($ty: $($trait)*,)*
        {
            type Return = U::Output;

            fn args() -> usize {
                $count
            }

            #[allow(clippy::drop_non_drop)]
            fn fn_call(&self, stack: &mut Stack, args: usize) -> VmResult<()> {
                drain_stack!($count, 0, stack, args, $($from_fn, $var, $num,)*);

                let fut = self($($var.0),*);
                // Note: we may drop any produced reference guard here since the
                // HRTB guarantees it won't escape nor be captured by the
                // future, so once the method returns we know they are no longer
                // used.
                $(drop($var.1);)*

                let ret = vm_try!(runtime::Future::new(async move {
                    let output = fut.await;
                    VmResult::Ok(vm_try!(output.to_value()))
                }));

                vm_try!(stack.push(vm_try!(Value::try_from(ret))));
                VmResult::Ok(())
            }
        }
    };
}

permute!(impl_function_traits);
repeat_macro!(impl_instance_function_traits);
