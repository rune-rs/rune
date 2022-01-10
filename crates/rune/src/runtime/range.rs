use crate::compile::{InstallWith, Named};
use crate::runtime::{
    FromValue, Iterator, Mut, Panic, RawMut, RawRef, RawStr, Ref, ToValue, UnsafeFromValue, Value,
    Vm, VmError, VmErrorKind,
};
use std::fmt;
use std::ops;

/// Struct representing a dynamic anonymous object.
///
/// # Examples
///
/// ```
/// use rune::ToValue;
/// use rune::runtime::{Range, RangeLimits};
///
/// # fn main() -> rune::Result<()> {
/// let from = 42i64.to_value()?;
/// let _ = Range::new(Some(from), None, RangeLimits::HalfOpen);
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct Range {
    /// The start value of the range.
    pub start: Option<Value>,
    /// The to value of the range.
    pub end: Option<Value>,
    /// The limits of the range.
    pub limits: RangeLimits,
}

impl Range {
    /// Construct a new range.
    pub fn new(start: Option<Value>, end: Option<Value>, limits: RangeLimits) -> Self {
        Self { start, end, limits }
    }

    /// Coerce range into an iterator.
    pub fn into_iterator(self) -> Result<Iterator, Panic> {
        match (self.limits, self.start, self.end) {
            (RangeLimits::HalfOpen, Some(Value::Integer(start)), Some(Value::Integer(end))) => {
                return Ok(Iterator::from_double_ended("std::ops::Range", start..end));
            }
            (RangeLimits::Closed, Some(Value::Integer(start)), Some(Value::Integer(end))) => {
                return Ok(Iterator::from_double_ended(
                    "std::ops::RangeToInclusive",
                    start..=end,
                ));
            }
            (_, Some(Value::Integer(start)), None) => {
                return Ok(Iterator::from("std::ops::RangeFrom", start..));
            }
            _ => (),
        }

        Err(Panic::custom("not an iterator"))
    }

    /// Value pointer equals implementation for a range.
    pub(crate) fn value_ptr_eq(vm: &mut Vm, a: &Self, b: &Self) -> Result<bool, VmError> {
        if a.limits != b.limits {
            return Ok(false);
        }

        match (&a.start, &b.start) {
            (None, None) => (),
            (Some(a), Some(b)) if Value::value_ptr_eq(vm, a, b)? => (),
            _ => return Ok(false),
        }

        match (&a.end, &b.end) {
            (None, None) => (),
            (Some(a), Some(b)) if Value::value_ptr_eq(vm, a, b)? => (),
            _ => return Ok(false),
        }

        Ok(true)
    }

    /// Test if the current range contains the given integer.
    pub(crate) fn contains_int(&self, n: i64) -> Result<bool, VmError> {
        let start: Option<i64> = match self.start.clone() {
            Some(value) => Some(FromValue::from_value(value)?),
            None => None,
        };

        let end: Option<i64> = match self.end.clone() {
            Some(value) => Some(FromValue::from_value(value)?),
            None => None,
        };

        let out = match self.limits {
            RangeLimits::HalfOpen => match (start, end) {
                (Some(start), Some(end)) => (start..end).contains(&n),
                (Some(start), None) => (start..).contains(&n),
                (None, Some(end)) => (..end).contains(&n),
                (None, None) => true,
            },
            RangeLimits::Closed => match (start, end) {
                (Some(start), Some(end)) => (start..=end).contains(&n),
                (None, Some(end)) => (..=end).contains(&n),
                _ => return Err(VmError::from(VmErrorKind::UnsupportedRange)),
            },
        };

        Ok(out)
    }
}

impl fmt::Debug for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(start) = &self.start {
            write!(f, "{:?}", start)?;
        }

        match self.limits {
            RangeLimits::HalfOpen => write!(f, "..")?,
            RangeLimits::Closed => write!(f, "..=")?,
        }

        if let Some(end) = &self.end {
            write!(f, "{:?}", end)?;
        }

        Ok(())
    }
}

/// The limits of a range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RangeLimits {
    /// A half-open range `..`.
    HalfOpen,
    /// A closed range `..=`.
    Closed,
}

/// Coercing `start..end` into a [Range].
impl<Idx> ToValue for ops::Range<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        let start = self.start.to_value()?;
        let end = self.end.to_value()?;
        let range = Range::new(Some(start), Some(end), RangeLimits::HalfOpen);
        Ok(Value::from(range))
    }
}

/// Coercing `start..` into a [Range].
impl<Idx> ToValue for ops::RangeFrom<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        let start = self.start.to_value()?;
        let range = Range::new(Some(start), None, RangeLimits::HalfOpen);
        Ok(Value::from(range))
    }
}

/// Coercing `..` into a [Range].
impl ToValue for ops::RangeFull {
    fn to_value(self) -> Result<Value, VmError> {
        let range = Range::new(None, None, RangeLimits::HalfOpen);
        Ok(Value::from(range))
    }
}

/// Coercing `start..=end` into a [Range].
impl<Idx> ToValue for ops::RangeInclusive<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        let (start, end) = self.into_inner();
        let start = start.to_value()?;
        let end = end.to_value()?;
        let range = Range::new(Some(start), Some(end), RangeLimits::Closed);
        Ok(Value::from(range))
    }
}

/// Coercing `..end` into a [Range].
impl<Idx> ToValue for ops::RangeTo<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        let end = self.end.to_value()?;
        let range = Range::new(None, Some(end), RangeLimits::HalfOpen);
        Ok(Value::from(range))
    }
}

/// Coercing `..=end` into a [Range].
impl<Idx> ToValue for ops::RangeToInclusive<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        let end = self.end.to_value()?;
        let range = Range::new(None, Some(end), RangeLimits::Closed);
        Ok(Value::from(range))
    }
}

impl FromValue for Range {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_range()?.take()?)
    }
}

impl FromValue for Mut<Range> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let object = value.into_range()?;
        let object = object.into_mut()?;
        Ok(object)
    }
}

impl FromValue for Ref<Range> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let object = value.into_range()?;
        let object = object.into_ref()?;
        Ok(object)
    }
}

impl UnsafeFromValue for &Range {
    type Output = *const Range;
    type Guard = RawRef;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let object = value.into_range()?;
        let object = object.into_ref()?;
        Ok(Ref::into_raw(object))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Range {
    type Output = *mut Range;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let object = value.into_range()?;
        let object = object.into_mut()?;
        Ok(Mut::into_raw(object))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}

impl Named for Range {
    const BASE_NAME: RawStr = RawStr::from_str("Range");
}

impl InstallWith for Range {}
