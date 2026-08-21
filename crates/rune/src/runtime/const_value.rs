#[macro_use]
mod macros;

mod node;

pub(crate) use self::node::{ConstBuilder, ConstNodeKind, ConstNodesError};
pub use self::node::{ConstFields, ConstFieldsIter, ConstValue, ConstValueBuf};

use core::any;
use core::cmp::Ordering;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::runtime;
use crate::{declare_dyn_trait, hash_in, Hash, TypeHash};

use super::{
    Bytes, ExpectedType, FromValue, Inline, Object, OwnedTuple, Repr, RuntimeError, ToValue, Tuple,
    Type, Value, VmErrorKind, VmIntegerRepr,
};

/// How deeply a [`ConstValue`] is allowed to nest.
///
/// A constant is stored as one array rather than as a tree, so nesting no
/// longer costs a native frame to build or to take apart. What it still costs
/// is everything which has to *understand* the nesting - lowering a constant
/// into instructions, turning one into a pattern - so how deep one may be is
/// still bounded, and the bound is checked once, where the array is built or
/// read back.
///
/// This is the ceiling the `max-const-depth` option is measured against. The
/// option can lower the effective bound but not raise it past this.
pub(crate) const MAX_CONST_DEPTH: usize = 128;

/// How many values a [`ConstValue`] is allowed to be made of.
///
/// The value a constant is converted from shares what it is made of - a value
/// used twice is one allocation pointed at twice - while a `ConstValue` is a
/// tree which owns each of its parts outright. So converting one expands a graph
/// into a tree, and a constant function which does nothing more suspicious than
/// `let v = f(n - 1); [v, v]` produces a value which is linear to evaluate and
/// exponential to convert. Depth does not catch it: that shape is only `n` deep.
///
/// Nothing else bounds it. `const-budget` bounds the instructions evaluation is
/// allowed to run, and the doubling above needs a handful per level.
pub(crate) const MAX_CONST_SIZE: usize = 1 << 16;
/// Derive for the [`ToConstValue`] trait.
///
/// This is principally used for associated constants in native modules, since
/// Rune has to be provided a constant-compatible method for constructing values
/// of the given type.
///
/// [`ToConstValue`]: trait@crate::ToConstValue
///
/// # Examples
///
/// ```
/// use rune::{docstring, Any, ContextError, Module, ToConstValue};
///
/// #[derive(Any, ToConstValue)]
/// pub struct Duration {
///     #[const_value(with = const_duration)]
///     inner: std::time::Duration,
/// }
///
/// mod const_duration {
///     use rune::runtime::{ConstValue, ConstValueBuf, RuntimeError, Value};
///     use std::time::Duration;
///
///     #[inline]
///     pub(super) fn to_const_value(duration: Duration) -> Result<ConstValueBuf, RuntimeError> {
///         let secs = duration.as_secs();
///         let nanos = duration.subsec_nanos();
///         rune::to_const_value((secs, nanos))
///     }
///
///     #[inline]
///     pub(super) fn from_const_value(value: &ConstValue) -> Result<Duration, RuntimeError> {
///         let (secs, nanos) = rune::from_const_value::<(u64, u32)>(value)?;
///         Ok(Duration::new(secs, nanos))
///     }
///
///     #[inline]
///     pub(super) fn from_value(value: Value) -> Result<Duration, RuntimeError> {
///         let (secs, nanos) = rune::from_value::<(u64, u32)>(value)?;
///         Ok(Duration::new(secs, nanos))
///     }
/// }
///
/// #[rune::module(::time)]
/// pub fn module() -> Result<Module, ContextError> {
///     let mut m = Module::from_meta(module__meta)?;
///     m.ty::<Duration>()?;
///
///     m
///         .constant(
///             "SECOND",
///             Duration {
///                 inner: std::time::Duration::from_secs(1),
///             },
///         )
///         .build_associated::<Duration>()?
///         .docs(docstring! {
///             /// The duration of one second.
///             ///
///             /// # Examples
///             ///
///             /// ```rune
///             /// use time::Duration;
///             ///
///             /// let duration = Duration::SECOND;
///             /// ```
///         })?;
///
///     Ok(m)
/// }
/// ```
pub use rune_macros::ToConstValue;

/// An array which does not describe a constant within the limits is reported
/// the way anything else the machine turns away is.
impl From<ConstNodesError> for RuntimeError {
    fn from(error: ConstNodesError) -> Self {
        match error {
            #[cfg(any(feature = "serde", feature = "musli"))]
            ConstNodesError::Empty | ConstNodesError::Malformed => {
                RuntimeError::new(VmErrorKind::MalformedConstValue)
            }
            ConstNodesError::TooDeep { max } => {
                RuntimeError::new(VmErrorKind::MaxConstDepth { max })
            }
            ConstNodesError::TooLarge { max } => {
                RuntimeError::new(VmErrorKind::MaxConstSize { max })
            }
            ConstNodesError::Alloc(error) => RuntimeError::from(error),
        }
    }
}

/// Convert something into a [`ConstValueBuf`].
///
/// # Examples
///
/// ```
/// let value = rune::to_const_value((i32::MIN, u64::MAX))?;
/// let (a, b) = rune::from_const_value::<(i32, u64)>(&value)?;
///
/// assert_eq!(a, i32::MIN);
/// assert_eq!(b, u64::MAX);
/// # Ok::<_, rune::support::Error>(())
/// ```
pub fn from_const_value<T>(value: impl AsRef<ConstValue>) -> Result<T, RuntimeError>
where
    T: FromConstValue,
{
    T::from_const_value(value.as_ref())
}

/// Convert something into a [`ConstValueBuf`].
///
/// # Examples
///
/// ```
/// let value = rune::to_const_value((i32::MIN, u64::MAX))?;
/// let (a, b) = rune::from_const_value::<(i32, u64)>(&value)?;
///
/// assert_eq!(a, i32::MIN);
/// assert_eq!(b, u64::MAX);
/// # Ok::<_, rune::support::Error>(())
/// ```
pub fn to_const_value(value: impl ToConstValue) -> Result<ConstValueBuf, RuntimeError> {
    value.to_const_value()
}

/// Trait to perform a conversion to a [`ConstValueBuf`].
pub trait ToConstValue: Sized {
    /// Convert into a constant value.
    fn to_const_value(self) -> Result<ConstValueBuf, RuntimeError>;

    /// Return the constant constructor for the given type.
    #[inline]
    #[doc(hidden)]
    fn construct() -> alloc::Result<Option<ConstConstructImpl>> {
        Ok(None)
    }
}

impl ToConstValue for ConstValueBuf {
    #[inline]
    fn to_const_value(self) -> Result<ConstValueBuf, RuntimeError> {
        Ok(self)
    }
}

impl ToConstValue for &ConstValue {
    #[inline]
    fn to_const_value(self) -> Result<ConstValueBuf, RuntimeError> {
        Ok(self.try_to_owned()?)
    }
}

impl ToConstValue for Value {
    #[inline]
    fn to_const_value(self) -> Result<ConstValueBuf, RuntimeError> {
        ConstValueBuf::from_value_ref(&self)
    }
}

impl ConstValueBuf {
    /// Construct a constant value that is a string.
    pub fn string(value: impl AsRef<str>) -> Result<ConstValueBuf, RuntimeError> {
        let value = alloc::Box::try_from(value.as_ref())?;
        Ok(Self::from_kind(ConstNodeKind::String(value)))
    }

    /// Construct a constant value that is bytes.
    pub fn bytes(value: impl AsRef<[u8]>) -> Result<ConstValueBuf, RuntimeError> {
        let value = alloc::Box::try_from(value.as_ref())?;
        Ok(Self::from_kind(ConstNodeKind::Bytes(value)))
    }

    /// Construct a new tuple constant value.
    pub fn tuple<I>(fields: I) -> Result<ConstValueBuf, RuntimeError>
    where
        I: IntoIterator,
        I::Item: AsRef<ConstValue>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::instance(OwnedTuple::HASH, Hash::EMPTY, fields)
    }

    /// Construct a constant value for a struct.
    pub fn for_struct<const N: usize>(
        hash: Hash,
        fields: [ConstValueBuf; N],
    ) -> Result<ConstValueBuf, RuntimeError> {
        Self::instance(hash, Hash::EMPTY, fields)
    }

    /// Construct an instance of `hash` out of the constants it is made of,
    /// which are laid down after it in the order they are given.
    pub(crate) fn instance<I>(
        hash: Hash,
        variant_hash: Hash,
        fields: I,
    ) -> Result<ConstValueBuf, RuntimeError>
    where
        I: IntoIterator,
        I::Item: AsRef<ConstValue>,
        I::IntoIter: ExactSizeIterator,
    {
        let fields = fields.into_iter();

        let mut builder = ConstBuilder::new();

        let at = builder.open(ConstNodeKind::Instance {
            hash,
            variant_hash,
            fields: fields.len() as u32,
        })?;

        for field in fields {
            builder.extend(field.as_ref())?;
        }

        builder.close(at);
        Ok(builder.build()?)
    }

    /// Construct a constant value from a reference to a value.
    ///
    /// The walk is bounded before it descends, which is what keeps a value
    /// whose depth a script decided from costing a native frame per level.
    pub(crate) fn from_value_ref(value: &Value) -> Result<ConstValueBuf, RuntimeError> {
        let mut builder = ConstBuilder::new();
        from_value_ref_at(value, 0, &mut builder)?;
        Ok(builder.build()?)
    }
}

/// Append the constant `value` describes to `builder`, having already descended
/// `depth` levels into the value the conversion started at.
fn from_value_ref_at(
    value: &Value,
    depth: usize,
    builder: &mut ConstBuilder,
) -> Result<(), RuntimeError> {
    if depth >= MAX_CONST_DEPTH {
        return Err(RuntimeError::new(VmErrorKind::MaxConstDepth {
            max: MAX_CONST_DEPTH,
        }));
    }

    // Counted here rather than per container so that what is bounded is
    // what is actually built, whatever shape it is in.
    if builder.len() >= MAX_CONST_SIZE {
        return Err(RuntimeError::new(VmErrorKind::MaxConstSize {
            max: MAX_CONST_SIZE,
        }));
    }

    let depth = depth + 1;

    match value.as_ref() {
        Repr::Inline(value) => {
            builder.leaf(ConstNodeKind::Inline(*value))?;
        }
        Repr::Dynamic(value) => {
            return Err(RuntimeError::from(VmErrorKind::ConstNotSupported {
                actual: value.type_info(),
            }));
        }
        Repr::Any(value) => match value.type_hash() {
            alloc::String::HASH => {
                let string = value.borrow_ref::<alloc::String>()?;
                builder.leaf(ConstNodeKind::String(alloc::Box::try_from(
                    string.as_str(),
                )?))?;
            }
            Bytes::HASH => {
                let bytes = value.borrow_ref::<Bytes>()?;
                builder.leaf(ConstNodeKind::Bytes(alloc::Box::try_from(
                    bytes.as_slice(),
                )?))?;
            }
            OwnedTuple::HASH => {
                let tuple = value.borrow_ref::<OwnedTuple>()?;

                let at = builder.open(ConstNodeKind::Instance {
                    hash: OwnedTuple::HASH,
                    variant_hash: Hash::EMPTY,
                    fields: tuple.len() as u32,
                })?;

                for value in tuple.iter() {
                    from_value_ref_at(value, depth, builder)?;
                }

                builder.close(at);
            }
            Object::HASH => {
                let object = value.borrow_ref::<Object>()?;

                let mut keys = alloc::Vec::try_with_capacity(object.len())?;

                for key in object.keys() {
                    keys.try_push(alloc::Box::try_from(key.as_str())?)?;
                }

                // The keys are stored in the order they are read back in, so
                // that nothing which walks a constant has to sort them again.
                keys.sort();

                let at = builder.open(ConstNodeKind::Object {
                    keys: keys.try_clone()?.try_into_boxed_slice()?,
                })?;

                for key in keys.iter() {
                    let Some(value) = object.get(key.as_ref()) else {
                        return Err(RuntimeError::new(VmErrorKind::MalformedConstValue));
                    };

                    from_value_ref_at(value, depth, builder)?;
                }

                builder.close(at);
            }
            Option::<Value>::HASH => {
                let option = value.borrow_ref::<Option<Value>>()?;

                match &*option {
                    Some(some) => {
                        let at = builder.open(ConstNodeKind::Instance {
                            hash: Option::<Value>::HASH,
                            variant_hash: hash_in!(crate, ::std::option::Option::Some),
                            fields: 1,
                        })?;

                        from_value_ref_at(some, depth, builder)?;
                        builder.close(at);
                    }
                    None => {
                        builder.leaf(ConstNodeKind::Instance {
                            hash: Option::<Value>::HASH,
                            variant_hash: hash_in!(crate, ::std::option::Option::None),
                            fields: 0,
                        })?;
                    }
                }
            }
            runtime::Vec::HASH => {
                let vec = value.borrow_ref::<runtime::Vec>()?;

                let at = builder.open(ConstNodeKind::Instance {
                    hash: runtime::Vec::HASH,
                    variant_hash: Hash::EMPTY,
                    fields: vec.len() as u32,
                })?;

                for value in vec.iter() {
                    from_value_ref_at(value, depth, builder)?;
                }

                builder.close(at);
            }
            _ => {
                return Err(RuntimeError::from(VmErrorKind::ConstNotSupported {
                    actual: value.type_info(),
                }));
            }
        },
    }

    Ok(())
}

impl ConstValue {
    /// Try to coerce the current value as the specified integer `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// let value = rune::to_const_value(u32::MAX)?;
    ///
    /// assert_eq!(value.as_integer::<u64>()?, u32::MAX as u64);
    /// assert!(value.as_integer::<i32>().is_err());
    ///
    /// # Ok::<(), rune::support::Error>(())
    /// ```
    pub fn as_integer<T>(&self) -> Result<T, RuntimeError>
    where
        T: TryFrom<i64> + TryFrom<u64>,
    {
        match self.kind() {
            ConstNodeKind::Inline(Inline::Signed(value)) => match (*value).try_into() {
                Ok(number) => Ok(number),
                Err(..) => Err(RuntimeError::new(
                    VmErrorKind::ValueToIntegerCoercionError {
                        from: VmIntegerRepr::from(*value),
                        to: any::type_name::<T>(),
                    },
                )),
            },
            ConstNodeKind::Inline(Inline::Unsigned(value)) => match (*value).try_into() {
                Ok(number) => Ok(number),
                Err(..) => Err(RuntimeError::new(
                    VmErrorKind::ValueToIntegerCoercionError {
                        from: VmIntegerRepr::from(*value),
                        to: any::type_name::<T>(),
                    },
                )),
            },
            kind => Err(RuntimeError::new(VmErrorKind::ExpectedNumber {
                actual: kind.type_info(),
            })),
        }
    }

    inline_macros!(inline_into);

    /// Coerce into the string this is, if it is one.
    pub fn as_string(&self) -> Result<&str, ExpectedType> {
        let ConstNodeKind::String(value) = self.kind() else {
            return Err(ExpectedType::new::<alloc::String>(self.type_info()));
        };

        Ok(value)
    }

    /// Coerce into the fields of the tuple this is, if it is one.
    pub fn as_tuple(&self) -> Result<ConstFields<'_>, ExpectedType> {
        let ConstNodeKind::Instance {
            hash: OwnedTuple::HASH,
            variant_hash: Hash::EMPTY,
            ..
        } = self.kind()
        else {
            return Err(ExpectedType::new::<Tuple>(self.type_info()));
        };

        Ok(self.fields())
    }

    /// Convert into virtual machine value.
    ///
    /// We provide this associated method since a constant value can be
    /// converted into a value infallibly, which is not captured by the trait
    /// otherwise.
    ///
    /// The walk is a single pass over the array the constant is stored as, with
    /// the containers which are part way through kept on a work stack, so a
    /// constant which nests deeply costs memory rather than native frames.
    pub(crate) fn to_value_with(&self, cx: &dyn ConstContext) -> Result<Value, RuntimeError> {
        /// A container whose values are still being built.
        struct Frame<'a> {
            kind: FrameKind<'a>,
            remaining: usize,
            values: alloc::Vec<Value>,
        }

        enum FrameKind<'a> {
            Tuple,
            Vec,
            Object(&'a [alloc::Box<str>]),
            Some,
        }

        fn close(frame: Frame<'_>) -> Result<Value, RuntimeError> {
            match frame.kind {
                FrameKind::Tuple => Ok(Value::try_from(OwnedTuple::try_from(frame.values)?)?),
                FrameKind::Vec => Ok(Value::try_from(runtime::Vec::from(frame.values))?),
                FrameKind::Object(keys) => {
                    let mut object = Object::with_capacity(keys.len())?;

                    for (key, value) in keys.iter().zip(frame.values) {
                        object.insert(alloc::String::try_from(key.as_ref())?, value)?;
                    }

                    Ok(Value::try_from(object)?)
                }
                FrameKind::Some => {
                    let mut values = frame.values.into_iter();

                    let Some(value) = values.next() else {
                        return Err(RuntimeError::new(VmErrorKind::MalformedConstValue));
                    };

                    Ok(Value::try_from(Some(value))?)
                }
            }
        }

        fn open<'a>(kind: FrameKind<'a>, len: usize) -> Result<Frame<'a>, RuntimeError> {
            Ok(Frame {
                kind,
                remaining: len,
                values: alloc::Vec::try_with_capacity(len)?,
            })
        }

        let nodes = self.as_nodes();
        let mut frames = alloc::Vec::<Frame<'_>>::new();
        let mut index = 0;

        loop {
            let Some(node) = nodes.get(index) else {
                return Err(RuntimeError::new(VmErrorKind::MalformedConstValue));
            };

            // What the node produces outright, if anything - a node which opens
            // a container produces nothing until the container is closed.
            let mut produced = None;

            match &node.kind {
                ConstNodeKind::Inline(value) => {
                    produced = Some(Value::from(*value));
                    index += 1;
                }
                ConstNodeKind::String(string) => {
                    produced = Some(Value::try_from(string.as_ref())?);
                    index += 1;
                }
                ConstNodeKind::Bytes(bytes) => {
                    produced = Some(Value::try_from(bytes.as_ref())?);
                    index += 1;
                }
                ConstNodeKind::Object { keys } => {
                    frames.try_push(open(FrameKind::Object(keys), keys.len())?)?;
                    index += 1;
                }
                ConstNodeKind::Instance {
                    hash,
                    variant_hash,
                    fields,
                } => {
                    let fields = *fields as usize;

                    match (*hash, *variant_hash) {
                        (OwnedTuple::HASH, Hash::EMPTY) => {
                            frames.try_push(open(FrameKind::Tuple, fields)?)?;
                            index += 1;
                        }
                        (runtime::Vec::HASH, Hash::EMPTY) => {
                            frames.try_push(open(FrameKind::Vec, fields)?)?;
                            index += 1;
                        }
                        (Option::<Value>::HASH, variant_hash) => {
                            match (variant_hash, fields) {
                                (hash_in!(crate, ::std::option::Option::Some), 1) => {
                                    frames.try_push(open(FrameKind::Some, 1)?)?;
                                }
                                (hash_in!(crate, ::std::option::Option::None), 0) => {
                                    produced = Some(Value::try_from(None)?);
                                }
                                _ => {
                                    return Err(RuntimeError::missing_constant_constructor(*hash));
                                }
                            }

                            index += 1;
                        }
                        (hash, _) => {
                            // A type which is only known to whoever declared it
                            // builds itself out of the constants it is made of,
                            // so its subtree is handed over whole rather than
                            // walked into here.
                            let Some(constructor) = cx.get(hash) else {
                                return Err(RuntimeError::missing_constant_constructor(hash));
                            };

                            let size = node.size as usize;

                            let Some(subtree) = nodes.get(index..index + size) else {
                                return Err(RuntimeError::new(VmErrorKind::MalformedConstValue));
                            };

                            produced =
                                Some(constructor.const_construct(ConstValue::from_nodes(subtree))?);

                            index += size;
                        }
                    }
                }
            }

            // Hand what was produced to the container which was waiting for it,
            // and close every container which that completed.
            loop {
                if let Some(value) = produced.take() {
                    let Some(frame) = frames.last_mut() else {
                        return Ok(value);
                    };

                    frame.values.try_push(value)?;
                    frame.remaining -= 1;
                }

                if !frames.last().is_some_and(|frame| frame.remaining == 0) {
                    break;
                }

                let Some(frame) = frames.pop() else {
                    break;
                };

                produced = Some(close(frame)?);
            }
        }
    }
}

impl FromValue for ConstValueBuf {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        ConstValueBuf::from_value_ref(&value)
    }
}

impl ToValue for ConstValueBuf {
    #[inline]
    fn to_value(self) -> Result<Value, RuntimeError> {
        ConstValue::to_value_with(&self, &EmptyConstContext)
    }
}

impl ConstValue {
    #[inline]
    #[cfg(test)]
    pub(crate) fn to_value(&self) -> Result<Value, RuntimeError> {
        self.to_value_with(&EmptyConstContext)
    }
}

impl AsRef<ConstValue> for ConstValue {
    #[inline]
    fn as_ref(&self) -> &ConstValue {
        self
    }
}

impl From<Inline> for ConstValueBuf {
    #[inline]
    fn from(value: Inline) -> Self {
        ConstValueBuf::from_kind(ConstNodeKind::Inline(value))
    }
}

impl TryFrom<alloc::String> for ConstValueBuf {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: alloc::String) -> Result<Self, Self::Error> {
        Ok(Self::from_kind(ConstNodeKind::String(
            alloc::Box::try_from(value)?,
        )))
    }
}

impl TryFrom<alloc::Box<str>> for ConstValueBuf {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: alloc::Box<str>) -> Result<Self, Self::Error> {
        Ok(Self::from_kind(ConstNodeKind::String(value)))
    }
}

impl TryFrom<Bytes> for ConstValueBuf {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl TryFrom<&str> for ConstValueBuf {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self::from_kind(ConstNodeKind::String(
            alloc::Box::try_from(value)?,
        )))
    }
}

impl ToConstValue for &str {
    #[inline]
    fn to_const_value(self) -> Result<ConstValueBuf, RuntimeError> {
        Ok(ConstValueBuf::try_from(self)?)
    }
}

impl TryFrom<alloc::Box<[u8]>> for ConstValueBuf {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: alloc::Box<[u8]>) -> Result<Self, Self::Error> {
        Ok(Self::from_kind(ConstNodeKind::Bytes(value)))
    }
}

impl TryFrom<&[u8]> for ConstValueBuf {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self::from_kind(ConstNodeKind::Bytes(alloc::Box::try_from(
            value,
        )?)))
    }
}

impl ToConstValue for &[u8] {
    #[inline]
    fn to_const_value(self) -> Result<ConstValueBuf, RuntimeError> {
        Ok(ConstValueBuf::try_from(self)?)
    }
}

/// Trait to perform a conversion from a [`ConstValue`].
pub trait FromConstValue: Sized {
    /// Convert from a constant value.
    fn from_const_value(value: &ConstValue) -> Result<Self, RuntimeError>;
}

impl FromConstValue for ConstValueBuf {
    #[inline]
    fn from_const_value(value: &ConstValue) -> Result<Self, RuntimeError> {
        Ok(value.try_to_owned()?)
    }
}

impl FromConstValue for bool {
    #[inline]
    fn from_const_value(value: &ConstValue) -> Result<Self, RuntimeError> {
        value.as_bool()
    }
}

impl FromConstValue for char {
    #[inline]
    fn from_const_value(value: &ConstValue) -> Result<Self, RuntimeError> {
        value.as_char()
    }
}

macro_rules! impl_integer {
    ($($ty:ty),* $(,)?) => {
        $(
            impl FromConstValue for $ty {
                #[inline]
                fn from_const_value(value: &ConstValue) -> Result<Self, RuntimeError> {
                    value.as_integer()
                }
            }
        )*
    };
}

impl_integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

declare_dyn_trait! {
    /// The vtable for constant constructors.
    struct ConstConstructVtable;

    /// The implementation wrapper for a constant constructor.
    pub struct ConstConstructImpl;

    /// Implementation of a constant constructor.
    ///
    /// Do not implement manually, this is provided when deriving [`ToConstValue`].
    ///
    /// [`ToConstValue`]: derive@ToConstValue
    pub trait ConstConstruct {
        /// Construct from the constant which describes the instance, whose
        /// fields are the subtrees it is made of.
        #[doc(hidden)]
        fn const_construct(&self, value: &ConstValue) -> Result<Value, RuntimeError>;

        /// Construct from values.
        #[doc(hidden)]
        fn runtime_construct(&self, fields: &mut [Value]) -> Result<Value, RuntimeError>;
    }
}

pub(crate) trait ConstContext {
    fn get(&self, hash: Hash) -> Option<&ConstConstructImpl>;
}

pub(crate) struct EmptyConstContext;

impl ConstContext for EmptyConstContext {
    #[inline]
    fn get(&self, _: Hash) -> Option<&ConstConstructImpl> {
        None
    }
}
