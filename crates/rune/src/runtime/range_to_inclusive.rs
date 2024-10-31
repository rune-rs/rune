use core::cmp::Ordering;
use core::fmt;
use core::ops;

use crate as rune;
use crate::alloc::clone::TryClone;
use crate::runtime::{
    EnvProtocolCaller, FromValue, ProtocolCaller, RuntimeError, ToValue, Value, VmResult,
};
use crate::Any;

/// Type for an inclusive range expression `..=end`.
///
/// # Examples
///
/// ```rune
/// let range = ..=10;
/// assert!(range.contains(-10));
/// assert!(range.contains(5));
/// assert!(range.contains(10));
/// assert!(!range.contains(20));
///
/// assert!(range is std::ops::RangeToInclusive);
/// ```
///
/// Ranges can contain any type:
///
/// ```rune
/// let range = ..='f';
/// assert_eq!(range.end, 'f');
/// range.end = 'g';
/// assert_eq!(range.end, 'g');
/// ```
///
/// # Examples
///
/// ```rust
/// use rune::runtime::RangeToInclusive;
///
/// let end = rune::to_value(10)?;
/// let _ = RangeToInclusive::new(end);
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Any, Clone, TryClone)]
#[rune(constructor, static_type = RANGE_TO_INCLUSIVE)]
#[rune(item = ::std::ops)]
pub struct RangeToInclusive {
    /// The end value of the range.
    #[rune(get, set)]
    pub end: Value,
}

impl RangeToInclusive {
    /// Construct a new range.
    pub fn new(end: Value) -> Self {
        Self { end }
    }

    /// Test the range for partial equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = ..='e';
    /// assert!(range == (..='e'));
    /// assert!(range != (..='f'));
    ///
    /// let range = ..=2.0;
    /// assert!(range == (..=2.0));
    /// assert!(range != (..=f64::NAN));
    /// assert!((..=f64::NAN) != (..=f64::NAN));
    /// ```
    #[rune::function(keep, protocol = PARTIAL_EQ)]
    pub fn partial_eq(&self, other: &Self) -> VmResult<bool> {
        self.partial_eq_with(other, &mut EnvProtocolCaller)
    }

    pub(crate) fn partial_eq_with(
        &self,
        b: &Self,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<bool> {
        Value::partial_eq_with(&self.end, &b.end, caller)
    }

    /// Test the range for total equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::eq;
    ///
    /// let range = ..='e';
    /// assert!(eq(range, ..='e'));
    /// assert!(!eq(range, ..='f'));
    /// ```
    #[rune::function(keep, protocol = EQ)]
    pub fn eq(&self, other: &Self) -> VmResult<bool> {
        self.eq_with(other, &mut EnvProtocolCaller)
    }

    pub(crate) fn eq_with(&self, b: &Self, caller: &mut dyn ProtocolCaller) -> VmResult<bool> {
        Value::eq_with(&self.end, &b.end, caller)
    }

    /// Test the range for partial ordering.
    ///
    /// # Examples
    ///
    /// ```rune
    /// assert!((..='a') < (..='b'));
    /// assert!((..='d') > (..='b'));
    /// assert!(!((..=f64::NAN) > (..=f64::INFINITY)));
    /// assert!(!((..=f64::NAN) < (..=f64::INFINITY)));
    /// ```
    #[rune::function(keep, protocol = PARTIAL_CMP)]
    pub fn partial_cmp(&self, other: &Self) -> VmResult<Option<Ordering>> {
        self.partial_cmp_with(other, &mut EnvProtocolCaller)
    }

    pub(crate) fn partial_cmp_with(
        &self,
        b: &Self,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<Option<Ordering>> {
        Value::partial_cmp_with(&self.end, &b.end, caller)
    }

    /// Test the range for total ordering.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::cmp;
    /// use std::cmp::Ordering;
    ///
    /// assert_eq!(cmp(..='a', ..='b'), Ordering::Less);
    /// assert_eq!(cmp(..='c', ..='b'), Ordering::Greater);
    /// ```
    #[rune::function(keep, protocol = CMP)]
    pub fn cmp(&self, other: &Self) -> VmResult<Ordering> {
        self.cmp_with(other, &mut EnvProtocolCaller)
    }

    pub(crate) fn cmp_with(&self, b: &Self, caller: &mut dyn ProtocolCaller) -> VmResult<Ordering> {
        Value::cmp_with(&self.end, &b.end, caller)
    }

    /// Test if the range contains the given value.
    ///
    /// The check is performed using the [`PARTIAL_CMP`] protocol.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = ..=10;
    ///
    /// assert!(range.contains(-10));
    /// assert!(range.contains(5));
    /// assert!(range.contains(10));
    /// assert!(!range.contains(20));
    ///
    /// assert!(range is std::ops::RangeToInclusive);
    /// ```
    #[rune::function(keep)]
    pub(crate) fn contains(&self, value: Value) -> VmResult<bool> {
        self.contains_with(value, &mut EnvProtocolCaller)
    }

    pub(crate) fn contains_with(
        &self,
        value: Value,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<bool> {
        VmResult::Ok(matches!(
            vm_try!(Value::partial_cmp_with(&self.end, &value, caller)),
            Some(Ordering::Greater | Ordering::Equal)
        ))
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
    fn to_value(self) -> Result<Value, RuntimeError> {
        let end = self.end.to_value()?;
        Ok(Value::new(RangeToInclusive::new(end))?)
    }
}

impl<Idx> FromValue for ops::RangeToInclusive<Idx>
where
    Idx: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        let range = value.into_any::<RangeToInclusive>()?;
        let end = Idx::from_value(range.end)?;
        Ok(ops::RangeToInclusive { end })
    }
}
