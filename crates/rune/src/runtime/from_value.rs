use core::cmp::Ordering;

use crate::alloc::{self, String};
use crate::Any;

use super::{
    AnyObj, ConstValue, FromConstValue, Mut, RawAnyGuard, Ref, RuntimeError, Value, VmResult,
};

/// Derive macro for the [`FromValue`] trait for converting types from the
/// dynamic `Value` container.
///
/// # Examples
///
/// ```
/// use rune::{FromValue, Vm};
/// use std::sync::Arc;
///
/// #[derive(FromValue)]
/// struct Foo {
///     field: u64,
/// }
///
/// let mut sources = rune::sources! {
///     entry => {
///         pub fn main() {
///             #{field: 42}
///         }
///     }
/// };
///
/// let unit = rune::prepare(&mut sources).build()?;
///
/// let mut vm = Vm::without_runtime(Arc::new(unit));
/// let foo = vm.call(["main"], ())?;
/// let foo: Foo = rune::from_value(foo)?;
///
/// assert_eq!(foo.field, 42);
/// # Ok::<_, rune::support::Error>(())
/// ```
pub use rune_macros::FromValue;

/// Cheap conversion trait to convert something infallibly into a dynamic [`Value`].
pub trait IntoValue {
    /// Convert into a dynamic [`Value`].
    #[doc(hidden)]
    fn into_value(self) -> Value;
}

impl IntoValue for Value {
    #[inline]
    fn into_value(self) -> Value {
        self
    }
}

impl IntoValue for &Value {
    #[inline]
    fn into_value(self) -> Value {
        self.clone()
    }
}

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
pub fn from_value<T>(value: impl IntoValue) -> Result<T, RuntimeError>
where
    T: FromValue,
{
    T::from_value(value.into_value())
}

/// Trait for converting types from the dynamic [Value] container.
///
/// # Examples
///
/// ```
/// use rune::{FromValue, Vm};
/// use std::sync::Arc;
///
/// #[derive(FromValue)]
/// struct Foo {
///     field: u64,
/// }
///
/// let mut sources = rune::sources!(entry => {
///     pub fn main() { #{field: 42} }
/// });
///
/// let unit = rune::prepare(&mut sources).build()?;
///
/// let mut vm = Vm::without_runtime(Arc::new(unit));
/// let foo = vm.call(["main"], ())?;
/// let foo: Foo = rune::from_value(foo)?;
///
/// assert_eq!(foo.field, 42);
/// # Ok::<_, rune::support::Error>(())
/// ```
pub trait FromValue: 'static + Sized {
    /// Try to convert to the given type, from the given value.
    fn from_value(value: Value) -> Result<Self, RuntimeError>;
}

/// Unsafe to mut coercion.
pub trait UnsafeToMut {
    /// The raw guard returned.
    ///
    /// Must only be dropped *after* the value returned from this function is no
    /// longer live.
    type Guard: 'static;

    /// # Safety
    ///
    /// Caller must ensure that the returned reference does not outlive the
    /// guard.
    unsafe fn unsafe_to_mut<'a>(value: Value) -> VmResult<(&'a mut Self, Self::Guard)>;
}

/// Unsafe to ref coercion.
pub trait UnsafeToRef {
    /// The raw guard returned.
    ///
    /// Must only be dropped *after* the value returned from this function is no
    /// longer live.
    type Guard: 'static;

    /// # Safety
    ///
    /// Caller must ensure that the returned reference does not outlive the
    /// guard.
    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)>;
}

/// A potentially unsafe conversion for value conversion.
///
/// This trait is used to convert values to references, which can be safely used
/// while an external function call is used. That sort of use is safe because we
/// hold onto the guard returned by the conversion during external function
/// calls.
#[deprecated = "Rune: Implementing this trait will no longer work. Use UnsafeToRef and UnsafeToMut instead."]
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
    fn unsafe_from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)>;

    /// Coerce the output of an unsafe from value into the final output type.
    ///
    /// # Safety
    ///
    /// The return value of this function may only be used while a virtual
    /// machine is not being modified.
    ///
    /// You must also make sure that the returned value does not outlive the
    /// guard.
    unsafe fn unsafe_coerce(output: Self::Output) -> Self;
}

impl<T> FromValue for T
where
    T: Any,
{
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.into_any()
    }
}

impl<T> FromValue for Mut<T>
where
    T: Any,
{
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.into_any_mut()
    }
}

impl<T> FromValue for Ref<T>
where
    T: Any,
{
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.into_any_ref()
    }
}

impl FromValue for AnyObj {
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.into_any_obj()
    }
}

impl FromValue for Value {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        Ok(value)
    }
}

// Option impls

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        Ok(match &*value.into_option_ref()? {
            Some(some) => Some(T::from_value(some.clone())?),
            None => None,
        })
    }
}

from_value_ref!(Option<Value>, into_option_ref, into_option_mut, into_option);

impl FromValue for ::rust_alloc::string::String {
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        let string = String::from_value(value)?;
        let string = ::rust_alloc::string::String::from(string);
        Ok(string)
    }
}

impl FromValue for alloc::Box<str> {
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        let string = value.borrow_string_ref()?;
        let string = alloc::Box::try_from(string.as_ref())?;
        Ok(string)
    }
}

#[cfg(feature = "alloc")]
impl FromValue for ::rust_alloc::boxed::Box<str> {
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        let string = value.borrow_string_ref()?;
        let string = ::rust_alloc::boxed::Box::<str>::from(string.as_ref());
        Ok(string)
    }
}

impl FromValue for Ref<str> {
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        Ok(Ref::map(Ref::<String>::from_value(value)?, String::as_str))
    }
}

impl UnsafeToRef for str {
    type Guard = RawAnyGuard;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        let string = vm_try!(value.into_any_ref::<String>());
        let (string, guard) = Ref::into_raw(string);
        VmResult::Ok((string.as_ref().as_str(), guard))
    }
}

impl UnsafeToMut for str {
    type Guard = RawAnyGuard;

    unsafe fn unsafe_to_mut<'a>(value: Value) -> VmResult<(&'a mut Self, Self::Guard)> {
        let string = vm_try!(value.into_any_mut::<String>());
        let (mut string, guard) = Mut::into_raw(string);
        VmResult::Ok((string.as_mut().as_mut_str(), guard))
    }
}

impl UnsafeToRef for String {
    type Guard = RawAnyGuard;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        let string = vm_try!(value.into_any_ref::<String>());
        let (string, guard) = Ref::into_raw(string);
        VmResult::Ok((string.as_ref(), guard))
    }
}

impl UnsafeToMut for String {
    type Guard = RawAnyGuard;

    unsafe fn unsafe_to_mut<'a>(value: Value) -> VmResult<(&'a mut Self, Self::Guard)> {
        let string = vm_try!(value.into_any_mut::<String>());
        let (mut string, guard) = Mut::into_raw(string);
        VmResult::Ok((string.as_mut(), guard))
    }
}

impl<T, E> FromValue for Result<T, E>
where
    T: FromValue,
    E: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        Ok(match &*value.into_result_ref()? {
            Ok(ok) => Result::Ok(T::from_value(ok.clone())?),
            Err(err) => Result::Err(E::from_value(err.clone())?),
        })
    }
}

from_value_ref!(Result<Value, Value>, into_result_ref, into_result_mut, into_result);

impl FromValue for bool {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.as_bool()
    }
}

impl FromConstValue for bool {
    #[inline]
    fn from_const_value(value: ConstValue) -> Result<Self, RuntimeError> {
        value.as_bool()
    }
}

impl FromValue for char {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.as_char()
    }
}

impl FromConstValue for char {
    #[inline]
    fn from_const_value(value: ConstValue) -> Result<Self, RuntimeError> {
        value.as_char()
    }
}

macro_rules! impl_integer {
    ($($ty:ty),* $(,)?) => {
        $(
            impl FromValue for $ty {
                #[inline]
                fn from_value(value: Value) -> Result<Self, RuntimeError> {
                    value.as_integer()
                }
            }

            impl FromConstValue for $ty {
                #[inline]
                fn from_const_value(value: ConstValue) -> Result<Self, RuntimeError> {
                    value.as_integer()
                }
            }
        )*
    };
}

impl_integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

impl FromValue for f64 {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.as_float()
    }
}

impl FromValue for f32 {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        Ok(value.as_float()? as f32)
    }
}

cfg_std! {
    macro_rules! impl_map {
        ($ty:ty, $key:ty) => {
            impl<T> FromValue for $ty
            where
                T: FromValue,
            {
                fn from_value(value: Value) -> Result<Self, RuntimeError> {
                    let object = value.into_object()?;

                    let mut output = <$ty>::with_capacity(object.len());

                    for (key, value) in object {
                        let key = <$key>::try_from(key)?;
                        let value = <T>::from_value(value)?;
                        output.insert(key, value);
                    }

                    Ok(output)
                }
            }
        };
    }

    impl_map!(::std::collections::HashMap<String, T>, String);
    impl_map!(::std::collections::HashMap<::rust_alloc::string::String, T>, ::rust_alloc::string::String);
}

macro_rules! impl_try_map {
    ($ty:ty, $key:ty) => {
        impl<T> FromValue for $ty
        where
            T: FromValue,
        {
            fn from_value(value: Value) -> Result<Self, RuntimeError> {
                let object = value.into_object()?;

                let mut output = <$ty>::try_with_capacity(object.len())?;

                for (key, value) in object {
                    let key = <$key>::try_from(key)?;
                    let value = <T>::from_value(value)?;
                    output.try_insert(key, value)?;
                }

                Ok(output)
            }
        }
    };
}

impl_try_map!(alloc::HashMap<String, T>, String);
#[cfg(feature = "alloc")]
impl_try_map!(alloc::HashMap<::rust_alloc::string::String, T>, ::rust_alloc::string::String);

impl FromValue for Ordering {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.as_ordering()
    }
}
