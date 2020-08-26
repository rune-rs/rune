use crate::stack::Stack;
use crate::value::{Value, ValueType, ValueTypeInfo, VecTuple};
use crate::vm::VmError;

mod bytes;
mod hash_map;
mod object;
mod option;
mod primitive;
mod result;
mod string;
mod tuple;
mod vec;

/// Trait for converting arguments into values.
pub trait IntoArgs {
    /// Encode arguments into a stack.
    ///
    /// # Safety
    ///
    /// This has the ability to encode references into the stack.
    /// The caller must ensure that the stack is cleared with
    /// [clear][Stack::clear] before the references are no longer valid.
    unsafe fn into_args(self, stack: &mut Stack) -> Result<(), VmError>;

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
    fn to_value(self) -> Result<Value, VmError>;
}

/// Trait for unsafe conversion of value types into values.
pub trait UnsafeToValue {
    /// Convert into a value, loading it into the specified virtual machine.
    ///
    /// # Safety
    ///
    /// The caller of this function need to make sure that the value converted
    /// doesn't outlive the virtual machine which uses it, since it might be
    /// encoded as a raw pointer in the slots of the virtual machine.
    unsafe fn unsafe_to_value(self) -> Result<Value, VmError>;
}

/// Trait for converting from a value.
pub trait FromValue: Sized {
    /// Try to convert to the given type, from the given value.
    fn from_value(value: Value) -> Result<Self, VmError>;
}

/// A potentially unsafe conversion for value conversion.
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
    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError>;

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
    T: 'static + FromValue,
{
    type Output = T;
    type Guard = ();

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self, Self::Guard), VmError> {
        Ok((T::from_value(value)?, ()))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        output
    }
}

impl<T> UnsafeToValue for T
where
    T: ToValue,
{
    unsafe fn unsafe_to_value(self) -> Result<Value, VmError> {
        self.to_value()
    }
}

impl FromValue for Value {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.clone())
    }
}

impl ToValue for Value {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(self)
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
            $($ty: UnsafeToValue + std::fmt::Debug,)*
        {
            #[allow(unused)]
            unsafe fn into_args(self, stack: &mut Stack) -> Result<(), VmError> {
                let ($($value,)*) = self;
                impl_into_args!(@push stack, [$($value)*]);
                Ok(())
            }

            fn count() -> usize {
                $count
            }
        }
    };

    (@push $stack:ident, [] $($value:ident)*) => {
        $(
            let $value = $value.unsafe_to_value()?;
            $stack.push($value);
        )*
    };

    (@push $vm:ident, [$first:ident $($rest:ident)*] $($value:ident)*) => {
        impl_into_args!(@push $vm, [$($rest)*] $first $($value)*)
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

macro_rules! impl_from_value_tuple_vec {
    () => {
    };

    ({$ty:ident, $value:ident, $count:expr}, $({$l_ty:ident, $l_value:ident, $l_count:expr},)*) => {
        impl_from_value_tuple_vec!{@impl $count, {$ty, $value, $count}, $({$l_ty, $l_value, $l_count},)*}
        impl_from_value_tuple_vec!{$({$l_ty, $l_value, $l_count},)*}
    };

    (@impl $count:expr, $({$ty:ident, $value:ident, $ignore_count:expr},)*) => {
        impl<$($ty,)*> FromValue for VecTuple<($($ty,)*)>
        where
            $($ty: FromValue,)*
        {
            fn from_value(value: Value) -> Result<Self, VmError> {
                let vec = value.into_vec()?;
                let vec = vec.take()?;

                if vec.len() != $count {
                    return Err(VmError::ExpectedTupleLength {
                        actual: vec.len(),
                        expected: $count,
                    });
                }

                #[allow(unused_mut, unused_variables)]
                let mut it = vec.into_iter();

                $(
                    let $value: $ty = match it.next() {
                        Some(value) => <$ty>::from_value(value)?,
                        None => {
                            return Err(VmError::IterationError);
                        },
                    };
                )*

                Ok(VecTuple(($($value,)*)))
            }
        }
    };
}

impl_from_value_tuple_vec!(
    {H, h, 8},
    {G, g, 7},
    {F, f, 6},
    {E, e, 5},
    {D, d, 4},
    {C, c, 3},
    {B, b, 2},
    {A, a, 1},
);
