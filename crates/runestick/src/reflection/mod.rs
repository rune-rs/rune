use crate::{Stack, Value, ValueError, ValueType, ValueTypeInfo, VmError};

mod bytes;
mod hash_map;
mod object;
mod option;
mod primitive;
mod result;
mod string;
mod tuple;
mod vec;

/// Trait for converting arguments into values unsafely.
///
/// This has the ability to encode references.
pub trait IntoArgs {
    /// Encode arguments into a stack.
    ///
    /// # Safety
    ///
    /// This has the ability to encode references into the stack.
    /// The caller must ensure that the stack is cleared with
    /// [clear][Stack::clear] before the references are no longer valid.
    fn into_args(self, stack: &mut Stack) -> Result<(), VmError>;

    /// Convert arguments into a vector.
    fn into_vec(self) -> Result<Vec<Value>, VmError>;

    /// The number of arguments.
    fn count() -> usize;
}

/// Trait for converting types into values.
pub trait ReflectValueType: Sized {
    /// The internal, owned type used for this value.
    type Owned;

    /// Convert into a value type.
    fn value_type() -> ValueType;

    /// Access diagnostical information on the value type.
    fn value_type_info() -> ValueTypeInfo;
}

/// Trait for converting types into values.
pub trait ToValue: Sized {
    /// Convert into a value.
    fn to_value(self) -> Result<Value, ValueError>;
}

/// Trait for converting from a value.
pub trait FromValue: 'static + Sized {
    /// Try to convert to the given type, from the given value.
    fn from_value(value: Value) -> Result<Self, ValueError>;
}

/// A potentially unsafe conversion for value conversion.
///
/// This trait is specifically implemented for reference types to allow
/// registered functions to take references to their inner value.
///
/// This is specifically safe, because a guard is always held to the reference.
pub trait UnsafeFromValue: Sized {
    /// The output type from the unsafe coercion.
    type Output: 'static;

    /// The raw guard returned.
    ///
    /// Must only be dropped *after* the value returned from this function is
    /// no longer live.
    type Guard: 'static;

    /// Convert the given reference using unsafe assumptions to a value.
    ///
    /// # Safety
    ///
    /// The return value of this function may only be used while a virtual
    /// machine is not being modified.
    ///
    /// You must also make sure that the returned value does not outlive the
    /// guard.
    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError>;

    /// Coerce the output of an unsafe from value into the final output type.
    ///
    /// # Safety
    ///
    /// The return value of this function may only be used while a virtual
    /// machine is not being modified.
    ///
    /// You must also make sure that the returned value does not outlive the
    /// guard.
    unsafe fn to_arg(output: Self::Output) -> Self;
}

impl<T> UnsafeFromValue for T
where
    T: FromValue,
{
    type Output = T;
    type Guard = ();

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self, Self::Guard), ValueError> {
        Ok((T::from_value(value)?, ()))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        output
    }
}

impl FromValue for Value {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(value)
    }
}

impl<T> ToValue for T
where
    Value: From<T>,
{
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::from(self))
    }
}

impl ToValue for &Value {
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(self.clone())
    }
}

macro_rules! impl_into_args {
    () => {
        impl_into_args!{@impl 0,}
    };

    ({$ty:ident, $value:ident, $count:expr}, $({$l_ty:ident, $l_value:ident, $l_count:expr},)*) => {
        impl_into_args!{@impl $count, {$ty, $value, $count}, $({$l_ty, $l_value, $l_count},)*}
        impl_into_args!{$({$l_ty, $l_value, $l_count},)*}
    };

    (@impl $count:expr, $({$ty:ident, $value:ident, $ignore_count:expr},)*) => {
        impl<$($ty,)*> IntoArgs for ($($ty,)*)
        where
            $($ty: ToValue + std::fmt::Debug,)*
        {
            #[allow(unused)]
            fn into_args(self, stack: &mut Stack) -> Result<(), VmError> {
                let ($($value,)*) = self;
                $(stack.push($value.to_value()?);)*
                Ok(())
            }

            #[allow(unused)]
            fn into_vec(self) -> Result<Vec<Value>, VmError> {
                let ($($value,)*) = self;
                $(let $value = <$ty>::to_value($value)?;)*
                Ok(vec![$($value,)*])
            }

            fn count() -> usize {
                $count
            }
        }
    };
}

impl_into_args!(
    {H, h, 8},
    {G, g, 7},
    {F, f, 6},
    {E, e, 5},
    {D, d, 4},
    {C, c, 3},
    {B, b, 2},
    {A, a, 1},
);
