use core::cmp::Ordering;
use core::fmt;
use core::ops;

use crate as rune;
use crate::compile::Named;
use crate::module::InstallWith;
use crate::runtime::{
    FromValue, Iterator, ProtocolCaller, RawStr, ToValue, Value, VmErrorKind, VmResult,
};

/// Struct representing a dynamic anonymous object.
///
/// # Examples
///
/// ```
/// use rune::runtime::Range;
///
/// let start = rune::to_value(1)?;
/// let end = rune::to_value(10)?;
/// let _ = Range::new(start, end);
/// # Ok::<_, rune::Error>(())
/// ```
#[derive(Clone)]
pub struct Range {
    /// The start value of the range.
    pub start: Value,
    /// The to value of the range.
    pub end: Value,
}

impl Range {
    /// Construct a new range.
    pub fn new(start: Value, end: Value) -> Self {
        Self { start, end }
    }

    /// Iterate over the range.
    ///
    /// # Panics
    ///
    /// This panics if the range is not a well-defined range.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = 'a'..'e';
    /// assert_eq!(range.iter().collect::<Vec>(), ['a', 'b', 'c', 'd']);
    /// ```
    ///
    /// Cannot construct an iterator over floats:
    ///
    /// ```rune,should_panic
    /// let range = 1.0..2.0;
    /// range.iter()
    /// ```
    #[rune::function(keep)]
    pub fn iter(&self) -> VmResult<Iterator> {
        const NAME: &str = "std::ops::Range";

        match (&self.start, &self.end) {
            (Value::Byte(start), Value::Byte(end)) => {
                VmResult::Ok(Iterator::from_double_ended(NAME, *start..*end))
            }
            (Value::Char(start), Value::Char(end)) => {
                VmResult::Ok(Iterator::from_double_ended(NAME, *start..*end))
            }
            (Value::Integer(start), Value::Integer(end)) => {
                VmResult::Ok(Iterator::from_double_ended(NAME, *start..*end))
            }
            (start, end) => VmResult::err(VmErrorKind::UnsupportedIterRange {
                start: vm_try!(start.type_info()),
                end: vm_try!(end.type_info()),
            }),
        }
    }

    pub(crate) fn partial_eq_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        if !vm_try!(Value::partial_eq_with(&a.start, &b.start, caller)) {
            return VmResult::Ok(false);
        }

        Value::partial_eq_with(&a.end, &b.end, caller)
    }

    pub(crate) fn eq_with(a: &Self, b: &Self, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        if !vm_try!(Value::eq_with(&a.start, &b.start, caller)) {
            return VmResult::Ok(false);
        }

        Value::eq_with(&a.end, &b.end, caller)
    }

    pub(crate) fn cmp_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Ordering> {
        match vm_try!(Value::cmp_with(&a.start, &b.start, caller)) {
            Ordering::Equal => (),
            other => return VmResult::Ok(other),
        }

        Value::cmp_with(&a.end, &b.end, caller)
    }

    /// Test if the range contains the given integer.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = 0..10;
    ///
    /// assert!(!range.contains::<i64>(-10));
    /// assert!(range.contains::<i64>(5));
    /// assert!(!range.contains::<i64>(10));
    /// assert!(!range.contains::<i64>(20));
    ///
    /// assert!(range is std::ops::Range);
    /// ```
    #[rune::function(path = contains::<i64>)]
    pub(crate) fn contains(&self, n: i64) -> VmResult<bool> {
        let start: i64 = vm_try!(FromValue::from_value(self.start.clone()));
        let end: i64 = vm_try!(FromValue::from_value(self.end.clone()));
        VmResult::Ok((start..end).contains(&n))
    }
}

impl fmt::Debug for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}..{:?}", self.start, self.end)
    }
}

impl<Idx> ToValue for ops::Range<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> VmResult<Value> {
        let start = vm_try!(self.start.to_value());
        let end = vm_try!(self.end.to_value());
        let range = Range::new(start, end);
        VmResult::Ok(Value::from(range))
    }
}

impl<Idx> FromValue for ops::Range<Idx>
where
    Idx: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        let range = vm_try!(vm_try!(value.into_range()).take());
        let start = vm_try!(Idx::from_value(range.start));
        let end = vm_try!(Idx::from_value(range.end));
        VmResult::Ok(ops::Range { start, end })
    }
}

from_value!(Range, into_range);

impl Named for Range {
    const BASE_NAME: RawStr = RawStr::from_str("Range");
}

impl InstallWith for Range {}
