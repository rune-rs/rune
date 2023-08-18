use core::fmt;
use core::ops;

use crate as rune;
use crate::compile::Named;
use crate::module::InstallWith;
use crate::runtime::{
    FromValue, Iterator, Panic, ProtocolCaller, RawStr, ToValue, Value, VmErrorKind, VmResult,
};

/// Struct representing a dynamic anonymous object.
///
/// # Examples
///
/// ```
/// use rune::runtime::{Range, RangeLimits};
///
/// let from = rune::to_value(42i64)?;
/// let _ = Range::new(Some(from), None, RangeLimits::HalfOpen);
/// # Ok::<_, rune::Error>(())
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
    pub fn into_iterator(self) -> VmResult<Iterator> {
        match (self.limits, self.start, self.end) {
            (RangeLimits::HalfOpen, Some(start), Some(end)) => {
                const NAME: &str = "std::ops::Range";

                match (start, end) {
                    (Value::Integer(start), Value::Integer(end)) => {
                        return VmResult::Ok(Iterator::from_double_ended(NAME, start..end));
                    }
                    (Value::Byte(start), Value::Byte(end)) => {
                        return VmResult::Ok(Iterator::from_double_ended(NAME, start..end));
                    }
                    _ => {}
                }
            }
            (RangeLimits::Closed, Some(start), Some(end)) => {
                const NAME: &str = "std::ops::RangeToInclusive";

                match (start, end) {
                    (Value::Integer(start), Value::Integer(end)) => {
                        return VmResult::Ok(Iterator::from_double_ended(NAME, start..=end));
                    }
                    (Value::Byte(start), Value::Byte(end)) => {
                        return VmResult::Ok(Iterator::from_double_ended(NAME, start..=end));
                    }
                    _ => {}
                }
            }
            (_, Some(start), None) => {
                const NAME: &str = "std::ops::RangeFrom";

                match start {
                    Value::Integer(start) => {
                        return VmResult::Ok(Iterator::from(NAME, start..));
                    }
                    Value::Byte(start) => {
                        return VmResult::Ok(Iterator::from(NAME, start..));
                    }
                    _ => {}
                }
            }
            _ => (),
        }

        VmResult::err(Panic::custom("Not an iterator"))
    }

    /// Value pointer equals implementation for a range.
    pub(crate) fn eq_with(a: &Self, b: &Self, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        if a.limits != b.limits {
            return VmResult::Ok(false);
        }

        match (&a.start, &b.start) {
            (None, None) => (),
            (Some(a), Some(b)) if vm_try!(Value::eq_with(a, b, caller)) => (),
            _ => return VmResult::Ok(false),
        }

        match (&a.end, &b.end) {
            (None, None) => (),
            (Some(a), Some(b)) if vm_try!(Value::eq_with(a, b, caller)) => (),
            _ => return VmResult::Ok(false),
        }

        VmResult::Ok(true)
    }

    /// Test if the range contains the given integer.
    #[rune::function(keep, path = contains::<i64>)]
    pub(crate) fn contains_int(&self, n: i64) -> VmResult<bool> {
        let start: Option<i64> = match self.start.clone() {
            Some(value) => Some(vm_try!(FromValue::from_value(value))),
            None => None,
        };

        let end: Option<i64> = match self.end.clone() {
            Some(value) => Some(vm_try!(FromValue::from_value(value))),
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
                _ => return VmResult::err(VmErrorKind::UnsupportedRange),
            },
        };

        VmResult::Ok(out)
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
    fn to_value(self) -> VmResult<Value> {
        let start = vm_try!(self.start.to_value());
        let end = vm_try!(self.end.to_value());
        let range = Range::new(Some(start), Some(end), RangeLimits::HalfOpen);
        VmResult::Ok(Value::from(range))
    }
}

/// Coercing `start..` into a [Range].
impl<Idx> ToValue for ops::RangeFrom<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> VmResult<Value> {
        let start = vm_try!(self.start.to_value());
        let range = Range::new(Some(start), None, RangeLimits::HalfOpen);
        VmResult::Ok(Value::from(range))
    }
}

/// Coercing `..` into a [Range].
impl ToValue for ops::RangeFull {
    fn to_value(self) -> VmResult<Value> {
        let range = Range::new(None, None, RangeLimits::HalfOpen);
        VmResult::Ok(Value::from(range))
    }
}

/// Coercing `start..=end` into a [Range].
impl<Idx> ToValue for ops::RangeInclusive<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> VmResult<Value> {
        let (start, end) = self.into_inner();
        let start = vm_try!(start.to_value());
        let end = vm_try!(end.to_value());
        let range = Range::new(Some(start), Some(end), RangeLimits::Closed);
        VmResult::Ok(Value::from(range))
    }
}

/// Coercing `..end` into a [Range].
impl<Idx> ToValue for ops::RangeTo<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> VmResult<Value> {
        let end = vm_try!(self.end.to_value());
        let range = Range::new(None, Some(end), RangeLimits::HalfOpen);
        VmResult::Ok(Value::from(range))
    }
}

/// Coercing `..=end` into a [Range].
impl<Idx> ToValue for ops::RangeToInclusive<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> VmResult<Value> {
        let end = vm_try!(self.end.to_value());
        let range = Range::new(None, Some(end), RangeLimits::Closed);
        VmResult::Ok(Value::from(range))
    }
}

from_value!(Range, into_range);

impl Named for Range {
    const BASE_NAME: RawStr = RawStr::from_str("Range");
}

impl InstallWith for Range {}
