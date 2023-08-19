use core::fmt;
use core::ops;

use crate as rune;
use crate::compile::Named;
use crate::module::InstallWith;
use crate::runtime::{FromValue, ProtocolCaller, RawStr, ToValue, Value, VmResult};

/// Struct representing an open range `..=end`.
///
/// # Examples
///
/// ```
/// use rune::runtime::RangeToInclusive;
///
/// let end = rune::to_value(10)?;
/// let _ = RangeToInclusive::new(end);
/// # Ok::<_, rune::Error>(())
/// ```
#[derive(Clone)]
pub struct RangeToInclusive {
    /// The end value of the range.
    pub end: Value,
}

impl RangeToInclusive {
    /// Construct a new range.
    pub fn new(end: Value) -> Self {
        Self { end }
    }

    /// Value pointer equals implementation for a range.
    pub(crate) fn eq_with(a: &Self, b: &Self, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        if !vm_try!(Value::eq_with(&a.end, &b.end, caller)) {
            return VmResult::Ok(false);
        }

        VmResult::Ok(true)
    }

    /// Test if the range contains the given integer.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = ..=10;
    ///
    /// assert!(range.contains::<i64>(-10));
    /// assert!(range.contains::<i64>(5));
    /// assert!(range.contains::<i64>(10));
    /// assert!(!range.contains::<i64>(20));
    ///
    /// assert!(range is std::ops::RangeToInclusive);
    /// ```
    #[rune::function(path = contains::<i64>)]
    pub(crate) fn contains(&self, n: i64) -> VmResult<bool> {
        let end: i64 = vm_try!(FromValue::from_value(self.end.clone()));
        VmResult::Ok((..=end).contains(&n))
    }
}

impl fmt::Debug for RangeToInclusive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "..={:?}", self.end)
    }
}

impl<Idx> ToValue for ops::RangeToInclusive<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> VmResult<Value> {
        let end = vm_try!(self.end.to_value());
        VmResult::Ok(Value::from(RangeToInclusive::new(end)))
    }
}

impl<Idx> FromValue for ops::RangeToInclusive<Idx>
where
    Idx: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        let range = vm_try!(vm_try!(value.into_range_to_inclusive()).take());
        let end = vm_try!(Idx::from_value(range.end));
        VmResult::Ok(ops::RangeToInclusive { end })
    }
}

from_value!(RangeToInclusive, into_range_to_inclusive);

impl Named for RangeToInclusive {
    const BASE_NAME: RawStr = RawStr::from_str("RangeToInclusive");
}

impl InstallWith for RangeToInclusive {}
