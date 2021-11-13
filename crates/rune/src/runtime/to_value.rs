use crate::runtime::{AnyObj, Object, Panic, Shared, Value, VmError, VmErrorKind, VmIntegerRepr};
use crate::Any;

#[doc(inline)]
pub use rune_macros::ToValue;

/// Trait for converting types into the dynamic [Value] container.
///
/// # Examples
///
/// ```
/// use rune::{Context, FromValue, ToValue, Sources, Source, Vm};
/// use std::sync::Arc;
///
/// #[derive(ToValue)]
/// struct Foo {
///     field: u64,
/// }
///
/// # fn main() -> rune::Result<()> {
/// let context = Context::with_default_modules()?;
///
/// let mut sources = Sources::new();
/// sources.insert(Source::new("entry", "pub fn main(foo) { foo.field + 1 }"));
///
/// let unit = rune::prepare(&context, &mut sources).build()?;
///
/// let mut vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));
/// let foo = vm.call(&["main"], (Foo { field: 42 },))?;
/// let foo = u64::from_value(foo)?;
///
/// assert_eq!(foo, 43);
/// # Ok(()) }
/// ```
pub trait ToValue: Sized {
    /// Convert into a value.
    fn to_value(self) -> Result<Value, VmError>;
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
    unsafe fn unsafe_to_value(self) -> Result<(Value, Self::Guard), VmError>;
}

impl<T> ToValue for T
where
    T: Any,
{
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::from(AnyObj::new(self)))
    }
}

impl<T> UnsafeToValue for T
where
    T: ToValue,
{
    type Guard = ();

    unsafe fn unsafe_to_value(self) -> Result<(Value, Self::Guard), VmError> {
        Ok((self.to_value()?, ()))
    }
}

impl ToValue for &Value {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(self.clone())
    }
}

// Option impls

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::from(Shared::new(match self {
            Some(some) => {
                let value = some.to_value()?;
                Some(value)
            }
            None => None,
        })))
    }
}

// String impls

impl ToValue for Box<str> {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::from(Shared::new(self.to_string())))
    }
}

// Result impls

impl<T> ToValue for Result<T, Panic>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        match self {
            Ok(value) => Ok(value.to_value()?),
            Err(reason) => Err(VmError::from(VmErrorKind::Panic { reason })),
        }
    }
}

impl<T> ToValue for Result<T, VmError>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        match self {
            Ok(value) => Ok(value.to_value()?),
            Err(error) => Err(error),
        }
    }
}

impl<T, E> ToValue for Result<T, E>
where
    T: ToValue,
    E: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        Ok(match self {
            Ok(ok) => {
                let ok = ok.to_value()?;
                Value::from(Shared::new(Ok(ok)))
            }
            Err(err) => {
                let err = err.to_value()?;
                Value::from(Shared::new(Err(err)))
            }
        })
    }
}

// number impls

macro_rules! number_value_trait {
    ($ty:ty) => {
        impl ToValue for $ty {
            fn to_value(self) -> Result<Value, VmError> {
                use std::convert::TryInto as _;

                match self.try_into() {
                    Ok(number) => Ok(Value::Integer(number)),
                    Err(..) => Err(VmError::from(VmErrorKind::IntegerToValueCoercionError {
                        from: VmIntegerRepr::from(self),
                        to: std::any::type_name::<i64>(),
                    })),
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
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::Float(self as f64))
    }
}

// map impls

macro_rules! impl_map {
    ($ty:ty) => {
        impl<T> ToValue for $ty
        where
            T: ToValue,
        {
            fn to_value(self) -> Result<Value, VmError> {
                let mut output = Object::with_capacity(self.len());

                for (key, value) in self {
                    output.insert(key, value.to_value()?);
                }

                Ok(Value::from(Shared::new(output)))
            }
        }
    };
}

impl_map!(std::collections::HashMap<String, T>);
