use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap};
use crate::any::AnyMarker;

use super::{AnyObj, Object, RuntimeError, Value, VmResult};

/// Derive macro for the [`ToValue`] trait for converting types into the dynamic
/// `Value` container.
///
/// # Examples
///
/// ```
/// use rune::{ToValue, Vm};
/// use std::sync::Arc;
///
/// #[derive(ToValue)]
/// struct Foo {
///     field: u64,
/// }
///
/// let mut sources = rune::sources! {
///     entry => {
///         pub fn main(foo) {
///             foo.field + 1
///         }
///     }
/// };
///
/// let unit = rune::prepare(&mut sources).build()?;
///
/// let mut vm = Vm::without_runtime(Arc::new(unit));
/// let value = vm.call(["main"], (Foo { field: 42 },))?;
/// let value: u64 = rune::from_value(value)?;
///
/// assert_eq!(value, 43);
/// # Ok::<_, rune::support::Error>(())
/// ```
pub use rune_macros::ToValue;

/// Convert something into the dynamic [`Value`].
///
/// # Examples
///
/// ```
/// use rune::{ToValue, Vm};
/// use std::sync::Arc;
///
/// #[derive(ToValue)]
/// struct Foo {
///     field: u64,
/// }
///
/// let mut sources = rune::sources! {
///     entry => {
///         pub fn main(foo) {
///             foo.field + 1
///         }
///     }
/// };
///
/// let unit = rune::prepare(&mut sources).build()?;
///
/// let mut vm = Vm::without_runtime(Arc::new(unit));
/// let foo = vm.call(["main"], (Foo { field: 42 },))?;
/// let foo: u64 = rune::from_value(foo)?;
///
/// assert_eq!(foo, 43);
/// # Ok::<_, rune::support::Error>(())
/// ```
pub fn to_value(value: impl ToValue) -> Result<Value, RuntimeError> {
    value.to_value()
}

/// Trait for converting types into the dynamic [`Value`] container.
///
/// # Examples
///
/// ```
/// use rune::{ToValue, Vm};
/// use std::sync::Arc;
///
/// #[derive(ToValue)]
/// struct Foo {
///     field: u64,
/// }
///
/// let mut sources = rune::sources! {
///     entry => {
///         pub fn main(foo) {
///             foo.field + 1
///         }
///     }
/// };
///
/// let unit = rune::prepare(&mut sources).build()?;
///
/// let mut vm = Vm::without_runtime(Arc::new(unit));
/// let foo = vm.call(["main"], (Foo { field: 42 },))?;
/// let foo: u64 = rune::from_value(foo)?;
///
/// assert_eq!(foo, 43);
/// # Ok::<_, rune::support::Error>(())
/// ```
pub trait ToValue: Sized {
    /// Convert into a value.
    fn to_value(self) -> Result<Value, RuntimeError>;
}

/// Trait governing things that can be returned from native functions.
pub trait ToReturn: Sized {
    /// Convert something into a return value.
    fn to_return(self) -> VmResult<Value>;
}

impl<T> ToReturn for VmResult<T>
where
    T: ToValue,
{
    #[inline]
    fn to_return(self) -> VmResult<Value> {
        match self {
            VmResult::Ok(value) => VmResult::Ok(vm_try!(value.to_value())),
            VmResult::Err(error) => VmResult::Err(error),
        }
    }
}

impl<T> ToReturn for T
where
    T: ToValue,
{
    #[inline]
    fn to_return(self) -> VmResult<Value> {
        VmResult::Ok(vm_try!(T::to_value(self)))
    }
}

impl ToValue for Value {
    #[inline]
    fn to_value(self) -> Result<Value, RuntimeError> {
        Ok(self)
    }
}

/// Trait for converting types into values.
pub trait UnsafeToValue: Sized {
    /// The type used to guard the unsafe value conversion.
    type Guard: 'static;

    /// Convert into a value.
    ///
    /// # Safety
    ///
    /// The value returned must not be used after the guard associated with it
    /// has been dropped.
    unsafe fn unsafe_to_value(self) -> VmResult<(Value, Self::Guard)>;
}

impl<T> ToValue for T
where
    T: AnyMarker,
{
    #[inline]
    fn to_value(self) -> Result<Value, RuntimeError> {
        Ok(Value::from(AnyObj::new(self)?))
    }
}

impl<T> UnsafeToValue for T
where
    T: ToValue,
{
    type Guard = ();

    unsafe fn unsafe_to_value(self) -> VmResult<(Value, Self::Guard)> {
        VmResult::Ok((vm_try!(self.to_value()), ()))
    }
}

impl ToValue for &Value {
    #[inline]
    fn to_value(self) -> Result<Value, RuntimeError> {
        Ok(self.clone())
    }
}

// Option impls

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, RuntimeError> {
        let option = match self {
            Some(some) => Some(some.to_value()?),
            None => None,
        };

        Ok(Value::try_from(option)?)
    }
}

// String impls

impl ToValue for alloc::Box<str> {
    fn to_value(self) -> Result<Value, RuntimeError> {
        let this = alloc::String::from(self);
        Ok(Value::new(this)?)
    }
}

impl ToValue for &str {
    fn to_value(self) -> Result<Value, RuntimeError> {
        let this = alloc::String::try_from(self)?;
        Ok(Value::new(this)?)
    }
}

#[cfg(feature = "alloc")]
impl ToValue for ::rust_alloc::boxed::Box<str> {
    fn to_value(self) -> Result<Value, RuntimeError> {
        let this = self.try_to_string()?;
        Ok(Value::new(this)?)
    }
}

#[cfg(feature = "alloc")]
impl ToValue for ::rust_alloc::string::String {
    fn to_value(self) -> Result<Value, RuntimeError> {
        let string = alloc::String::try_from(self)?;
        Ok(Value::new(string)?)
    }
}

impl<T, E> ToValue for Result<T, E>
where
    T: ToValue,
    E: ToValue,
{
    fn to_value(self) -> Result<Value, RuntimeError> {
        let result = match self {
            Ok(ok) => Ok(ok.to_value()?),
            Err(err) => Err(err.to_value()?),
        };

        Ok(Value::try_from(result)?)
    }
}

// map impls

macro_rules! impl_map {
    ($ty:ty) => {
        impl<T> ToValue for $ty
        where
            T: ToValue,
        {
            fn to_value(self) -> Result<Value, RuntimeError> {
                let mut output = Object::with_capacity(self.len())?;

                for (key, value) in self {
                    let key = alloc::String::try_from(key)?;
                    let value = value.to_value()?;
                    output.insert(key, value)?;
                }

                Ok(Value::try_from(output)?)
            }
        }
    };
}

impl_map!(HashMap<::rust_alloc::string::String, T>);
impl_map!(HashMap<alloc::String, T>);

cfg_std! {
    impl_map!(::std::collections::HashMap<::rust_alloc::string::String, T>);
    impl_map!(::std::collections::HashMap<alloc::String, T>);
}
