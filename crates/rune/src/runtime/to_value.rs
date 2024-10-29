use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap};
use crate::runtime::{AnyObj, Object, Value, VmError, VmResult};
use crate::Any;

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
pub fn to_value<T>(value: T) -> Result<Value, VmError>
where
    T: ToValue,
{
    T::to_value(value).into_result()
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
    fn to_value(self) -> VmResult<Value>;
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

    /// Attempts to convert this UnsafeToValue into a ToValue, which is only
    /// possible if it is not a reference to an Any type.
    fn try_into_to_value(self) -> Option<impl ToValue>;
}

impl<T> ToValue for T
where
    T: Any,
{
    #[inline]
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(Value::from(vm_try!(AnyObj::new(self))))
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

    fn try_into_to_value(self) -> Option<impl ToValue> {
        Some(self)
    }
}

impl ToValue for &Value {
    #[inline]
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(self.clone())
    }
}

// Option impls

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(self) -> VmResult<Value> {
        let option = match self {
            Some(some) => Some(vm_try!(some.to_value())),
            None => None,
        };

        VmResult::Ok(vm_try!(Value::try_from(option)))
    }
}

// String impls

impl ToValue for alloc::Box<str> {
    fn to_value(self) -> VmResult<Value> {
        let this = alloc::String::from(self);
        VmResult::Ok(vm_try!(Value::try_from(this)))
    }
}

impl ToValue for &str {
    fn to_value(self) -> VmResult<Value> {
        let this = vm_try!(alloc::String::try_from(self));
        VmResult::Ok(vm_try!(Value::try_from(this)))
    }
}

#[cfg(feature = "alloc")]
impl ToValue for ::rust_alloc::boxed::Box<str> {
    fn to_value(self) -> VmResult<Value> {
        let this = vm_try!(self.try_to_string());
        VmResult::Ok(vm_try!(Value::try_from(this)))
    }
}

#[cfg(feature = "alloc")]
impl ToValue for ::rust_alloc::string::String {
    fn to_value(self) -> VmResult<Value> {
        let string = vm_try!(alloc::String::try_from(self));
        let value = vm_try!(Value::try_from(string));
        VmResult::Ok(value)
    }
}

impl<T> ToValue for VmResult<T>
where
    T: ToValue,
{
    #[inline]
    fn to_value(self) -> VmResult<Value> {
        match self {
            VmResult::Ok(value) => value.to_value(),
            VmResult::Err(error) => VmResult::Err(error),
        }
    }
}

impl<T, E> ToValue for Result<T, E>
where
    T: ToValue,
    E: ToValue,
{
    fn to_value(self) -> VmResult<Value> {
        let result = match self {
            Ok(ok) => Ok(vm_try!(ok.to_value())),
            Err(err) => Err(vm_try!(err.to_value())),
        };

        VmResult::Ok(vm_try!(Value::try_from(result)))
    }
}

// map impls

macro_rules! impl_map {
    ($ty:ty) => {
        impl<T> ToValue for $ty
        where
            T: ToValue,
        {
            fn to_value(self) -> VmResult<Value> {
                let mut output = vm_try!(Object::with_capacity(self.len()));

                for (key, value) in self {
                    let key = vm_try!(alloc::String::try_from(key));
                    vm_try!(output.insert(key, vm_try!(value.to_value())));
                }

                VmResult::Ok(vm_try!(Value::try_from(output)))
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
