#[macro_use]
mod macros;

use core::future::Future;
use core::mem::replace;

use crate::alloc;
use crate::compile::meta;
use crate::hash::Hash;
use crate::runtime::{
    self, Address, AnyTypeInfo, FromValue, IntoReturn, MaybeTypeOf, Memory, Output, RuntimeError,
    TypeHash, TypeOf, UnsafeToMut, UnsafeToRef, Value, VmError, VmErrorKind,
};

// Expand to function variable bindings.
macro_rules! access_memory {
    ($count:expr, $add:expr, $memory:ident, $addr:ident, $args:ident, $($from_fn:path, $var:ident, $num:expr),* $(,)?) => {
        if $args != $count + $add {
            return Err(VmError::new(VmErrorKind::BadArgumentCount {
                actual: $args,
                expected: $count + $add,
            }));
        }

        let [$($var,)*] = $memory.slice_at_mut($addr, $args)? else {
            unreachable!();
        };

        $(let $var = replace($var, Value::empty());)*

        $(
            let $var = match $from_fn($var) {
                Ok($var) => $var,
                Err(error) => {
                    return Err(VmError::with_error(VmError::from(error), VmErrorKind::BadArgument {
                        arg: $num,
                    }));
                }
            };
        )*
    };
}

/// Denotes the kind of a function, allowing the [`Function`] trait to be
/// implemented separately for plain and async functions.
pub trait FunctionKind {
    /// Indicates if the function is async.
    const IS_ASYNC: bool;
}

/// Marker for plain functions.
#[non_exhaustive]
pub struct Plain;

impl FunctionKind for Plain {
    const IS_ASYNC: bool = false;
}

/// Marker for async functions.
#[non_exhaustive]
pub struct Async;

impl FunctionKind for Async {
    const IS_ASYNC: bool = true;
}

/// Trait used to provide the [function][crate::module::Module::function]
/// function.
#[diagnostic::on_unimplemented(
    label = "#[derive(Any)] could be missing on the arguments or return value of `{Self}`"
)]
pub trait Function<A, K>: 'static + Send + Sync {
    /// The return type of the function.
    #[doc(hidden)]
    type Return;

    /// Get the number of arguments.
    #[doc(hidden)]
    const ARGS: usize;

    /// Perform the vm call.
    #[doc(hidden)]
    fn call(
        &self,
        memory: &mut dyn Memory,
        addr: Address,
        args: usize,
        out: Output,
    ) -> Result<(), VmError>;
}

/// Trait used to provide the [`associated_function`] function.
///
/// [`associated_function`]: crate::module::Module::associated_function
#[diagnostic::on_unimplemented(
    label = "#[derive(Any)] could be missing on the arguments or return value of `{Self}`"
)]
pub trait InstanceFunction<A, K>: 'static + Send + Sync {
    /// The type of the instance.
    #[doc(hidden)]
    type Instance: TypeOf;

    /// The return type of the function.
    #[doc(hidden)]
    type Return;

    /// Get the number of arguments.
    #[doc(hidden)]
    const ARGS: usize;

    /// Perform the vm call.
    #[doc(hidden)]
    fn call(
        &self,
        memory: &mut dyn Memory,
        addr: Address,
        args: usize,
        out: Output,
    ) -> Result<(), VmError>;
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

            const ARGS: usize  = <T as Function<(Instance, $($ty,)*), Kind>>::ARGS;

            #[inline]
            fn call(&self, memory: &mut dyn Memory, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
                Function::call(self, memory, addr, args, out)
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
    fn maybe_type_of() -> alloc::Result<meta::TypeHash> {
        T::maybe_type_of()
    }
}

impl<T> TypeHash for Ref<T>
where
    T: ?Sized + TypeHash,
{
    const HASH: Hash = T::HASH;
}

impl<T> TypeOf for Ref<T>
where
    T: ?Sized + TypeOf,
{
    const PARAMETERS: Hash = T::PARAMETERS;
    const STATIC_TYPE_INFO: AnyTypeInfo = T::STATIC_TYPE_INFO;
}

/// Zero-sized marker struct for mutable references.
pub struct Mut<T: ?Sized>(PhantomData<T>);

impl<T> MaybeTypeOf for Mut<T>
where
    T: ?Sized + MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> alloc::Result<meta::TypeHash> {
        T::maybe_type_of()
    }
}

impl<T> TypeHash for Mut<T>
where
    T: ?Sized + TypeHash,
{
    const HASH: Hash = T::HASH;
}

impl<T> TypeOf for Mut<T>
where
    T: ?Sized + TypeOf,
{
    const PARAMETERS: Hash = T::PARAMETERS;
    const STATIC_TYPE_INFO: AnyTypeInfo = T::STATIC_TYPE_INFO;
}

// Fake guard for owned values.
struct Guard;

#[inline(always)]
fn from_value<T>(value: Value) -> Result<(T, Guard), RuntimeError>
where
    T: FromValue,
{
    Ok((T::from_value(value)?, Guard))
}

#[inline(always)]
fn unsafe_to_ref<'a, T>(value: Value) -> Result<(&'a T, T::Guard), RuntimeError>
where
    T: ?Sized + UnsafeToRef,
{
    // SAFETY: these are only locally used in this module, and we ensure that
    // the guard requirement is met.
    unsafe { T::unsafe_to_ref(value) }
}

#[inline(always)]
fn unsafe_to_mut<'a, T>(value: Value) -> Result<(&'a mut T, T::Guard), RuntimeError>
where
    T: ?Sized + UnsafeToMut,
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
            U: IntoReturn,
            $($ty: $($trait)*,)*
        {
            type Return = U;

            const ARGS: usize = $count;

            #[allow(clippy::drop_non_drop)]
            fn call(&self, memory: &mut dyn Memory, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
                access_memory!($count, 0, memory, addr, args, $($from_fn, $var, $num,)*);

                // Safety: We hold a reference to memory, so we can guarantee
                // that it won't be modified.
                let ret = self($($var.0),*);
                $(drop($var.1);)*

                let value = IntoReturn::into_return(ret)?;
                memory.store(out, value)?;
                Ok(())
            }
        }

        impl<T, U, $($ty,)*> Function<($($place,)*), Async> for T
        where
            T: 'static + Send + Sync + Fn($($($mut)* $ty),*) -> U,
            U: 'static + Future<Output: IntoReturn>,
            $($ty: $($trait)*,)*
        {
            type Return = U::Output;

            const ARGS: usize = $count;

            #[allow(clippy::drop_non_drop)]
            fn call(&self, memory: &mut dyn Memory, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
                access_memory!($count, 0, memory, addr, args, $($from_fn, $var, $num,)*);

                let fut = self($($var.0),*);
                // Note: we may drop any produced reference guard here since the
                // HRTB guarantees it won't escape nor be captured by the
                // future, so once the method returns we know they are no longer
                // used.
                $(drop($var.1);)*

                let ret = runtime::Future::new(async move {
                    IntoReturn::into_return(fut.await)
                })?;

                let value = Value::try_from(ret)?;
                memory.store(out, value)?;
                Ok(())
            }
        }
    };
}

permute!(impl_function_traits);
repeat_macro!(impl_instance_function_traits);
