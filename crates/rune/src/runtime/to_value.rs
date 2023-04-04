use crate::runtime::{AnyObj, Object, Shared, Value, VmErrorKind, VmIntegerRepr, VmResult};
use crate::Any;

#[doc(inline)]
pub use rune_macros::ToValue;

/// Trait for converting types into the dynamic [Value] container.
///
/// # Examples
///
/// ```
/// use rune::{FromValue, ToValue, Vm};
/// use std::sync::Arc;
///
/// #[derive(ToValue)]
/// struct Foo {
///     field: u64,
/// }
///
/// # fn main() -> rune::Result<()> {
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
/// let foo = u64::from_value(foo)?;
///
/// assert_eq!(foo, 43);
/// # Ok(()) }
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
        VmResult::Ok(Value::from(AnyObj::new(self)))
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
        VmResult::Ok(Value::from(Shared::new(match self {
            Some(some) => {
                let value = vm_try!(some.to_value());
                Some(value)
            }
            None => None,
        })))
    }
}

// String impls

impl ToValue for Box<str> {
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(Value::from(Shared::new(self.to_string())))
    }
}

impl ToValue for &str {
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(Value::from(Shared::new(self.to_string())))
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
                Value::from(Shared::new(Ok(ok)))
            }
            Err(err) => {
                let err = vm_try!(err.to_value());
                Value::from(Shared::new(Err(err)))
            }
        })
    }
}

// number impls

macro_rules! number_value_trait {
    ($ty:ty) => {
        impl ToValue for $ty {
            fn to_value(self) -> VmResult<Value> {
                use std::convert::TryInto as _;

                match self.try_into() {
                    Ok(number) => VmResult::Ok(Value::Integer(number)),
                    Err(..) => VmResult::err(VmErrorKind::IntegerToValueCoercionError {
                        from: VmIntegerRepr::from(self),
                        to: std::any::type_name::<i64>(),
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
                let mut output = Object::with_capacity(self.len());

                for (key, value) in self {
                    output.insert(key, vm_try!(value.to_value()));
                }

                VmResult::Ok(Value::from(Shared::new(output)))
            }
        }
    };
}

impl_map!(std::collections::HashMap<String, T>);
