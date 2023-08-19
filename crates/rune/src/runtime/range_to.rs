use core::cmp::Ordering;
use core::fmt;
use core::ops;

use crate as rune;
use crate::compile::Named;
use crate::module::InstallWith;
use crate::runtime::{FromValue, ProtocolCaller, RawStr, ToValue, Value, VmResult};

/// Struct representing an open range `..end`.
///
/// # Examples
///
/// ```
/// use rune::runtime::RangeTo;
///
/// let end = rune::to_value(1)?;
/// let _ = RangeTo::new(end);
/// # Ok::<_, rune::Error>(())
/// ```
#[derive(Clone)]
pub struct RangeTo {
    /// The end value of the range.
    pub end: Value,
}

impl RangeTo {
    /// Construct a new range.
    pub fn new(end: Value) -> Self {
        Self { end }
    }

    pub(crate) fn partial_eq_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        Value::partial_eq_with(&a.end, &b.end, caller)
    }

    pub(crate) fn eq_with(a: &Self, b: &Self, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        Value::eq_with(&a.end, &b.end, caller)
    }

    pub(crate) fn partial_cmp_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Option<Ordering>> {
        Value::partial_cmp_with(&a.end, &b.end, caller)
    }

    pub(crate) fn cmp_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Ordering> {
        Value::cmp_with(&a.end, &b.end, caller)
    }

    /// Test if the range contains the given integer.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = ..10;
    ///
    /// assert!(range.contains::<i64>(-10));
    /// assert!(range.contains::<i64>(5));
    /// assert!(!range.contains::<i64>(10));
    /// assert!(!range.contains::<i64>(20));
    ///
    /// assert!(range is std::ops::RangeTo);
    /// ```
    #[rune::function(path = contains::<i64>)]
    pub(crate) fn contains(&self, n: i64) -> VmResult<bool> {
        let end: i64 = vm_try!(FromValue::from_value(self.end.clone()));
        VmResult::Ok((..end).contains(&n))
    }
}

impl fmt::Debug for RangeTo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "..{:?}", self.end)
    }
}

impl<Idx> ToValue for ops::RangeTo<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> VmResult<Value> {
        let end = vm_try!(self.end.to_value());
        VmResult::Ok(Value::from(RangeTo::new(end)))
    }
}

impl<Idx> FromValue for ops::RangeTo<Idx>
where
    Idx: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        let range = vm_try!(vm_try!(value.into_range_to()).take());
        let end = vm_try!(Idx::from_value(range.end));
        VmResult::Ok(ops::RangeTo { end })
    }
}

from_value!(RangeTo, into_range_to);

impl Named for RangeTo {
    const BASE_NAME: RawStr = RawStr::from_str("RangeTo");
}

impl InstallWith for RangeTo {}
