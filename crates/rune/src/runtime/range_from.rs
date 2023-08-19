use core::cmp::Ordering;
use core::fmt;
use core::ops;

use crate as rune;
use crate::compile::Named;
use crate::module::InstallWith;
use crate::runtime::{
    FromValue, Iterator, ProtocolCaller, RawStr, ToValue, Value, VmErrorKind, VmResult,
};

/// Struct representing an open range `start..`.
///
/// # Examples
///
/// ```
/// use rune::runtime::RangeFrom;
///
/// let start = rune::to_value(1)?;
/// let _ = RangeFrom::new(start);
/// # Ok::<_, rune::Error>(())
/// ```
#[derive(Clone)]
pub struct RangeFrom {
    /// The start value of the range.
    pub start: Value,
}

impl RangeFrom {
    /// Construct a new range.
    pub fn new(start: Value) -> Self {
        Self { start }
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
    /// let range = 'a'..;
    /// assert_eq!(range.iter().take(5).collect::<Vec>(), ['a', 'b', 'c', 'd', 'e']);
    /// ```
    ///
    /// Cannot construct an iterator over floats:
    ///
    /// ```rune,should_panic
    /// let range = 1.0..;
    /// range.iter()
    /// ```
    #[rune::function(keep)]
    pub fn iter(&self) -> VmResult<Iterator> {
        const NAME: &str = "std::ops::RangeFrom";

        match &self.start {
            Value::Byte(start) => VmResult::Ok(Iterator::from(NAME, *start..)),
            Value::Char(start) => VmResult::Ok(Iterator::from(NAME, *start..)),
            Value::Integer(start) => VmResult::Ok(Iterator::from(NAME, *start..)),
            start => VmResult::err(VmErrorKind::UnsupportedIterRangeFrom {
                start: vm_try!(start.type_info()),
            }),
        }
    }

    pub(crate) fn eq_with(a: &Self, b: &Self, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        VmResult::Ok(vm_try!(Value::eq_with(&a.start, &b.start, caller)))
    }

    pub(crate) fn cmp_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Ordering> {
        VmResult::Ok(vm_try!(Value::cmp_with(&a.start, &b.start, caller)))
    }

    /// Test if the range contains the given integer.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = 0..;
    ///
    /// assert!(!range.contains::<i64>(-10));
    /// assert!(range.contains::<i64>(5));
    /// assert!(range.contains::<i64>(10));
    /// assert!(range.contains::<i64>(20));
    ///
    /// assert!(range is std::ops::RangeFrom);
    /// ```
    #[rune::function(path = contains::<i64>)]
    pub(crate) fn contains(&self, n: i64) -> VmResult<bool> {
        let start: i64 = vm_try!(FromValue::from_value(self.start.clone()));
        VmResult::Ok((start..).contains(&n))
    }
}

impl fmt::Debug for RangeFrom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}..", self.start)
    }
}

impl<Idx> ToValue for ops::RangeFrom<Idx>
where
    Idx: ToValue,
{
    fn to_value(self) -> VmResult<Value> {
        let start = vm_try!(self.start.to_value());
        let range = RangeFrom::new(start);
        VmResult::Ok(Value::from(range))
    }
}

impl<Idx> FromValue for ops::RangeFrom<Idx>
where
    Idx: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        let range = vm_try!(vm_try!(value.into_range_from()).take());
        let start = vm_try!(Idx::from_value(range.start));
        VmResult::Ok(ops::RangeFrom { start })
    }
}

from_value!(RangeFrom, into_range_from);

impl Named for RangeFrom {
    const BASE_NAME: RawStr = RawStr::from_str("RangeFrom");
}

impl InstallWith for RangeFrom {}
