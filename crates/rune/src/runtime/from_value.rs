use core::cmp::Ordering;

use crate::alloc;
use crate::runtime::{
    AnyObj, Mut, RawMut, RawRef, Ref, Shared, Value, VmError, VmErrorKind, VmResult,
};
use crate::Any;

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
pub fn from_value<T>(value: Value) -> Result<T, VmError>
where
    T: FromValue,
{
    T::from_value(value).into_result()
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
    fn from_value(value: Value) -> VmResult<Self>;
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
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(vm_try!(value.into_any()).take_downcast()))
    }
}

impl<T> FromValue for Mut<T>
where
    T: Any,
{
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(vm_try!(value.into_any()).downcast_into_mut()))
    }
}

impl<T> FromValue for Ref<T>
where
    T: Any,
{
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(vm_try!(value.into_any()).downcast_into_ref()))
    }
}

impl FromValue for Shared<AnyObj> {
    fn from_value(value: Value) -> VmResult<Self> {
        value.into_any()
    }
}

impl FromValue for Value {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(value)
    }
}

// Option impls

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> VmResult<Self> {
        let option = vm_try!(value.into_option());
        let option = vm_try!(option.take());
        VmResult::Ok(match option {
            Some(some) => Some(vm_try!(T::from_value(some))),
            None => None,
        })
    }
}

impl UnsafeToRef for Option<Value> {
    type Guard = RawRef;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        let option = vm_try!(value.into_option());
        let option = vm_try!(option.into_ref());
        let (value, guard) = Ref::into_raw(option);
        VmResult::Ok((value.as_ref(), guard))
    }
}

impl UnsafeToMut for Option<Value> {
    type Guard = RawMut;

    unsafe fn unsafe_to_mut<'a>(value: Value) -> VmResult<(&'a mut Self, Self::Guard)> {
        let option = vm_try!(value.into_option());
        let option = vm_try!(option.into_mut());
        let (mut value, guard) = Mut::into_raw(option);
        VmResult::Ok((value.as_mut(), guard))
    }
}

impl FromValue for alloc::String {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::String(string) => VmResult::Ok(vm_try!(string.take())),
            actual => VmResult::err(VmErrorKind::expected::<alloc::String>(vm_try!(
                actual.type_info()
            ))),
        }
    }
}

impl FromValue for ::rust_alloc::string::String {
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(::rust_alloc::string::String::from(vm_try!(
            alloc::String::from_value(value)
        )))
    }
}

impl FromValue for alloc::Box<str> {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::String(string) => {
                let string = vm_try!(string.take());
                let string = vm_try!(string.try_into_boxed_str());
                VmResult::Ok(string)
            }
            actual => VmResult::err(VmErrorKind::expected::<alloc::String>(vm_try!(
                actual.type_info()
            ))),
        }
    }
}

#[cfg(feature = "alloc")]
impl FromValue for ::rust_alloc::boxed::Box<str> {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::String(string) => {
                let string = vm_try!(string.take());
                let string = ::rust_alloc::boxed::Box::from(string.as_str());
                VmResult::Ok(string)
            }
            actual => VmResult::err(VmErrorKind::expected::<alloc::String>(vm_try!(
                actual.type_info()
            ))),
        }
    }
}

impl FromValue for Mut<alloc::String> {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::String(string) => VmResult::Ok(vm_try!(string.into_mut())),
            actual => VmResult::err(VmErrorKind::expected::<alloc::String>(vm_try!(
                actual.type_info()
            ))),
        }
    }
}

impl FromValue for Ref<alloc::String> {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::String(string) => VmResult::Ok(vm_try!(string.into_ref())),
            actual => VmResult::err(VmErrorKind::expected::<alloc::String>(vm_try!(
                actual.type_info()
            ))),
        }
    }
}

impl FromValue for Ref<str> {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::String(string) => {
                VmResult::Ok(Ref::map(vm_try!(string.into_ref()), |s| s.as_str()))
            }
            actual => VmResult::err(VmErrorKind::expected::<alloc::String>(vm_try!(
                actual.type_info()
            ))),
        }
    }
}

impl UnsafeToRef for str {
    type Guard = RawRef;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        match value {
            Value::String(string) => {
                let string = vm_try!(string.into_ref());
                let (string, guard) = Ref::into_raw(string);
                VmResult::Ok((string.as_ref(), guard))
            }
            actual => VmResult::err(VmErrorKind::expected::<alloc::String>(vm_try!(
                actual.type_info()
            ))),
        }
    }
}

impl UnsafeToMut for str {
    type Guard = RawMut;

    unsafe fn unsafe_to_mut<'a>(value: Value) -> VmResult<(&'a mut Self, Self::Guard)> {
        match value {
            Value::String(string) => {
                let string = vm_try!(string.into_mut());
                let (mut string, guard) = Mut::into_raw(string);
                VmResult::Ok((string.as_mut().as_mut_str(), guard))
            }
            actual => VmResult::err(VmErrorKind::expected::<alloc::String>(vm_try!(
                actual.type_info()
            ))),
        }
    }
}

impl UnsafeToRef for alloc::String {
    type Guard = RawRef;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        match value {
            Value::String(string) => {
                let string = vm_try!(string.into_ref());
                let (string, guard) = Ref::into_raw(string);
                VmResult::Ok((string.as_ref(), guard))
            }
            actual => VmResult::err(VmErrorKind::expected::<alloc::String>(vm_try!(
                actual.type_info()
            ))),
        }
    }
}

impl UnsafeToMut for alloc::String {
    type Guard = RawMut;

    unsafe fn unsafe_to_mut<'a>(value: Value) -> VmResult<(&'a mut Self, Self::Guard)> {
        match value {
            Value::String(string) => {
                let string = vm_try!(string.into_mut());
                let (mut string, guard) = Mut::into_raw(string);
                VmResult::Ok((string.as_mut(), guard))
            }
            actual => VmResult::err(VmErrorKind::expected::<alloc::String>(vm_try!(
                actual.type_info()
            ))),
        }
    }
}

impl<T, E> FromValue for Result<T, E>
where
    T: FromValue,
    E: FromValue,
{
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(match vm_try!(vm_try!(value.into_result()).take()) {
            Ok(ok) => Result::Ok(vm_try!(T::from_value(ok))),
            Err(err) => Result::Err(vm_try!(E::from_value(err))),
        })
    }
}

impl UnsafeToRef for Result<Value, Value> {
    type Guard = RawRef;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        let result = vm_try!(value.into_result());
        let result = vm_try!(result.into_ref());
        let (value, guard) = Ref::into_raw(result);
        VmResult::Ok((value.as_ref(), guard))
    }
}

impl UnsafeToMut for Result<Value, Value> {
    type Guard = RawMut;

    unsafe fn unsafe_to_mut<'a>(value: Value) -> VmResult<(&'a mut Self, Self::Guard)> {
        let result = vm_try!(value.into_result());
        let result = vm_try!(result.into_mut());
        let (mut value, guard) = Mut::into_raw(result);
        VmResult::Ok((value.as_mut(), guard))
    }
}

impl FromValue for u8 {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(value.into_byte()))
    }
}

impl FromValue for bool {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(value.into_bool()))
    }
}

impl FromValue for char {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(value.into_char()))
    }
}

impl FromValue for i64 {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(value.into_integer()))
    }
}

macro_rules! impl_number {
    ($ty:ty) => {
        impl FromValue for $ty {
            #[inline]
            fn from_value(value: Value) -> VmResult<Self> {
                value.try_into_integer()
            }
        }
    };
}

impl_number!(u16);
impl_number!(u32);
impl_number!(u64);
impl_number!(u128);
impl_number!(usize);
impl_number!(i8);
impl_number!(i16);
impl_number!(i32);
impl_number!(i128);
impl_number!(isize);

impl FromValue for f64 {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        value.into_float()
    }
}

impl FromValue for f32 {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(value.into_float()) as f32)
    }
}

cfg_std! {
    macro_rules! impl_map {
        ($ty:ty, $key:ty) => {
            impl<T> FromValue for $ty
            where
                T: FromValue,
            {
                fn from_value(value: Value) -> VmResult<Self> {
                    let object = vm_try!(value.into_object());
                    let object = vm_try!(object.take());

                    let mut output = <$ty>::with_capacity(object.len());

                    for (key, value) in object {
                        let key = vm_try!(<$key>::try_from(key));
                        let value = vm_try!(<T>::from_value(value));
                        output.insert(key, value);
                    }

                    VmResult::Ok(output)
                }
            }
        };
    }

    impl_map!(::std::collections::HashMap<alloc::String, T>, alloc::String);
    impl_map!(::std::collections::HashMap<::rust_alloc::string::String, T>, ::rust_alloc::string::String);
}

macro_rules! impl_try_map {
    ($ty:ty, $key:ty) => {
        impl<T> FromValue for $ty
        where
            T: FromValue,
        {
            fn from_value(value: Value) -> VmResult<Self> {
                let object = vm_try!(value.into_object());
                let object = vm_try!(object.take());

                let mut output = vm_try!(<$ty>::try_with_capacity(object.len()));

                for (key, value) in object {
                    let key = vm_try!(<$key>::try_from(key));
                    let value = vm_try!(<T>::from_value(value));
                    vm_try!(output.try_insert(key, value));
                }

                VmResult::Ok(output)
            }
        }
    };
}

impl_try_map!(alloc::HashMap<alloc::String, T>, alloc::String);
#[cfg(feature = "alloc")]
impl_try_map!(alloc::HashMap<::rust_alloc::string::String, T>, ::rust_alloc::string::String);

impl FromValue for Ordering {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        value.into_ordering()
    }
}
