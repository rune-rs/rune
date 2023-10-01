use core::any;
use core::cmp::Ordering;

use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap};
use crate::runtime::{
    AnyObj, Object, Shared, Value, VmError, VmErrorKind, VmIntegerRepr, VmResult,
};
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
}

impl<T> ToValue for T
where
    T: Any,
{
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(vm_try!(Value::try_from(vm_try!(AnyObj::new(self)))))
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
        VmResult::Ok(Value::from(vm_try!(Shared::new(match self {
            Some(some) => Some(vm_try!(some.to_value())),
            None => None,
        }))))
    }
}

// String impls

impl ToValue for alloc::Box<str> {
    fn to_value(self) -> VmResult<Value> {
        let this = alloc::String::from(self);
        VmResult::Ok(Value::from(vm_try!(Shared::new(this))))
    }
}

impl ToValue for &str {
    fn to_value(self) -> VmResult<Value> {
        let this = vm_try!(alloc::String::try_from(self));
        VmResult::Ok(Value::from(vm_try!(Shared::new(this))))
    }
}

#[cfg(feature = "alloc")]
impl ToValue for ::rust_alloc::boxed::Box<str> {
    fn to_value(self) -> VmResult<Value> {
        let this = vm_try!(self.try_to_string());
        VmResult::Ok(Value::from(vm_try!(Shared::new(this))))
    }
}

#[cfg(feature = "alloc")]
impl ToValue for ::rust_alloc::string::String {
    fn to_value(self) -> VmResult<Value> {
        let string = vm_try!(alloc::String::try_from(self));
        VmResult::Ok(Value::from(vm_try!(Shared::new(string))))
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
        VmResult::Ok(match self {
            Ok(ok) => {
                let ok = vm_try!(ok.to_value());
                Value::from(vm_try!(Shared::new(Ok(ok))))
            }
            Err(err) => {
                let err = vm_try!(err.to_value());
                Value::from(vm_try!(Shared::new(Err(err))))
            }
        })
    }
}

// number impls

macro_rules! number_value_trait {
    ($ty:ty) => {
        impl ToValue for $ty {
            fn to_value(self) -> VmResult<Value> {
                match self.try_into() {
                    Ok(number) => VmResult::Ok(Value::Integer(number)),
                    Err(..) => VmResult::err(VmErrorKind::IntegerToValueCoercionError {
                        from: VmIntegerRepr::from(self),
                        to: any::type_name::<i64>(),
                    }),
                }
            }
        }
    };
}

number_value_trait!(u16);
number_value_trait!(u32);
number_value_trait!(u64);
number_value_trait!(u128);
number_value_trait!(usize);
number_value_trait!(i8);
number_value_trait!(i16);
number_value_trait!(i32);
number_value_trait!(i128);
number_value_trait!(isize);

impl ToValue for f32 {
    #[inline]
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(Value::Float(self as f64))
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

                VmResult::Ok(Value::from(vm_try!(Shared::new(output))))
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

impl ToValue for Ordering {
    #[inline]
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(Value::Ordering(self))
    }
}
