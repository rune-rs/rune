use crate::runtime::{
    AnyObj, Mut, RawMut, RawRef, Ref, Shared, StaticString, Value, VmErrorKind, VmIntegerRepr,
    VmResult,
};
use crate::Any;
use std::sync::Arc;

#[doc(inline)]
pub use rune_macros::FromValue;

/// Trait for converting types from the dynamic [Value] container.
///
/// # Examples
///
/// ```
/// use rune::{Context, FromValue, Sources, Source, Vm};
/// use std::sync::Arc;
///
/// #[derive(FromValue)]
/// struct Foo {
///     field: u64,
/// }
///
/// # fn main() -> rune::Result<()> {
/// let mut sources = rune::sources!(entry => {
///     pub fn main() { #{field: 42} }
/// });
///
/// let unit = rune::prepare(&mut sources).build()?;
///
/// let mut vm = Vm::without_runtime(Arc::new(unit));
/// let foo = vm.call(["main"], ())?;
/// let foo = Foo::from_value(foo)?;
///
/// assert_eq!(foo.field, 42);
/// # Ok(()) }
/// ```
pub trait FromValue: 'static + Sized {
    /// Try to convert to the given type, from the given value.
    fn from_value(value: Value) -> VmResult<Self>;
}

/// A potentially unsafe conversion for value conversion.
///
/// This trait is used to convert values to references, which can be safely used
/// while an external function call is used. That sort of use is safe because we
/// hold onto the guard returned by the conversion during external function
/// calls.
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
    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)>;

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

impl<T> UnsafeFromValue for T
where
    T: FromValue,
{
    type Output = T;
    type Guard = ();

    fn from_value(value: Value) -> VmResult<(Self, Self::Guard)> {
        VmResult::Ok((vm_try!(T::from_value(value)), ()))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        output
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

impl UnsafeFromValue for &Option<Value> {
    type Output = *const Option<Value>;
    type Guard = RawRef;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let option = vm_try!(value.into_option());
        let option = vm_try!(option.into_ref());
        VmResult::Ok(Ref::into_raw(option))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Option<Value> {
    type Output = *mut Option<Value>;
    type Guard = RawMut;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let option = vm_try!(value.into_option());
        let option = vm_try!(option.into_mut());
        VmResult::Ok(Mut::into_raw(option))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

impl UnsafeFromValue for &mut Result<Value, Value> {
    type Output = *mut Result<Value, Value>;
    type Guard = RawMut;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let result = vm_try!(value.into_result());
        let result = vm_try!(result.into_mut());
        VmResult::Ok(Mut::into_raw(result))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

// String impls

impl FromValue for String {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::String(string) => VmResult::Ok(vm_try!(string.borrow_ref()).clone()),
            Value::StaticString(string) => VmResult::Ok((**string).to_owned()),
            actual => VmResult::err(VmErrorKind::expected::<String>(vm_try!(actual.type_info()))),
        }
    }
}

impl FromValue for Mut<String> {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::String(string) => VmResult::Ok(vm_try!(string.into_mut())),
            actual => VmResult::err(VmErrorKind::expected::<String>(vm_try!(actual.type_info()))),
        }
    }
}

impl FromValue for Ref<String> {
    fn from_value(value: Value) -> VmResult<Self> {
        match value {
            Value::String(string) => VmResult::Ok(vm_try!(string.into_ref())),
            actual => VmResult::err(VmErrorKind::expected::<String>(vm_try!(actual.type_info()))),
        }
    }
}

impl FromValue for Box<str> {
    fn from_value(value: Value) -> VmResult<Self> {
        let string = vm_try!(value.into_string());
        let string = vm_try!(string.borrow_ref()).clone();
        VmResult::Ok(string.into_boxed_str())
    }
}

/// Raw guard used for `&str` references.
///
/// Note that we need to hold onto an instance of the static string to prevent
/// the reference to it from being deallocated (the `StaticString` variant).
pub enum StrGuard {
    RawRef(RawRef),
    StaticString(Arc<StaticString>),
}

impl UnsafeFromValue for &str {
    type Output = *const str;
    type Guard = StrGuard;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        VmResult::Ok(match value {
            Value::String(string) => {
                let string = vm_try!(string.into_ref());
                let (s, guard) = Ref::into_raw(string);
                // Safety: we're holding onto the guard for the string here, so
                // it is live.
                (unsafe { (*s).as_str() }, StrGuard::RawRef(guard))
            }
            Value::StaticString(string) => {
                (string.as_ref().as_str(), StrGuard::StaticString(string))
            }
            actual => {
                return VmResult::err(VmErrorKind::expected::<String>(vm_try!(actual.type_info())))
            }
        })
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut str {
    type Output = *mut str;
    type Guard = Option<RawMut>;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        match value {
            Value::String(string) => {
                let string = vm_try!(string.into_mut());
                let (s, guard) = Mut::into_raw(string);
                // Safety: we're holding onto the guard for the string here, so
                // it is live.
                VmResult::Ok((unsafe { (*s).as_mut_str() }, Some(guard)))
            }
            actual => VmResult::err(VmErrorKind::expected::<String>(vm_try!(actual.type_info()))),
        }
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

impl UnsafeFromValue for &String {
    type Output = *const String;
    type Guard = StrGuard;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        VmResult::Ok(match value {
            Value::String(string) => {
                let string = vm_try!(string.into_ref());
                let (s, guard) = Ref::into_raw(string);
                (s, StrGuard::RawRef(guard))
            }
            Value::StaticString(string) => (&**string, StrGuard::StaticString(string)),
            actual => {
                return VmResult::err(VmErrorKind::expected::<String>(vm_try!(actual.type_info())));
            }
        })
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut String {
    type Output = *mut String;
    type Guard = RawMut;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        VmResult::Ok(match value {
            Value::String(string) => {
                let string = vm_try!(string.into_mut());
                let (s, guard) = Mut::into_raw(string);
                (s, guard)
            }
            actual => {
                return VmResult::err(VmErrorKind::expected::<String>(vm_try!(actual.type_info())));
            }
        })
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

// Result impls

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

impl UnsafeFromValue for &Result<Value, Value> {
    type Output = *const Result<Value, Value>;
    type Guard = RawRef;

    fn from_value(value: Value) -> VmResult<(Self::Output, Self::Guard)> {
        let result = vm_try!(value.into_result());
        let result = vm_try!(result.into_ref());
        VmResult::Ok(Ref::into_raw(result))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

// number impls

impl FromValue for () {
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(value.into_unit()))
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
            fn from_value(value: Value) -> VmResult<Self> {
                use std::convert::TryInto as _;
                let integer = vm_try!(value.into_integer());

                match integer.try_into() {
                    Ok(number) => VmResult::Ok(number),
                    Err(..) => VmResult::err(VmErrorKind::ValueToIntegerCoercionError {
                        from: VmIntegerRepr::from(integer),
                        to: std::any::type_name::<Self>(),
                    }),
                }
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

// map impls

macro_rules! impl_map {
    ($ty:ty) => {
        impl<T> FromValue for $ty
        where
            T: FromValue,
        {
            fn from_value(value: Value) -> VmResult<Self> {
                let object = vm_try!(value.into_object());
                let object = vm_try!(object.take());

                let mut output = <$ty>::with_capacity(object.len());

                for (key, value) in object {
                    output.insert(key, vm_try!(T::from_value(value)));
                }

                VmResult::Ok(output)
            }
        }
    };
}

impl_map!(std::collections::HashMap<String, T>);
