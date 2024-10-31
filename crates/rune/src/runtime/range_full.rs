use core::cmp::Ordering;
use core::fmt;
use core::ops;

use crate as rune;
use crate::alloc::clone::TryClone;
use crate::runtime::{FromValue, RuntimeError, ToValue, Value, VmResult};
use crate::Any;

/// Type for a full range expression `..`.
///
/// # Examples
///
/// ```rune
/// let range = ..;
///
/// assert!(range.contains(-10));
/// assert!(range.contains(5));
/// assert!(range.contains(10));
/// assert!(range.contains(20));
///
/// assert!(range is std::ops::RangeFull);
/// ```
///
/// # Rust Examples
///
/// ```rust
/// use rune::runtime::RangeFull;
///
/// let _ = RangeFull::new();
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Any, Default, Clone, TryClone)]
#[try_clone(crate)]
#[rune(constructor, static_type = RANGE_FULL)]
#[rune(item = ::std::ops)]
pub struct RangeFull;

impl RangeFull {
    /// Construct a new full range.
    pub const fn new() -> Self {
        Self
    }

    /// Test if the range contains the given value.
    ///
    /// The check is performed using the [`PARTIAL_CMP`] protocol.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = ..;
    ///
    /// assert!(range.contains(-10));
    /// assert!(range.contains(5));
    /// assert!(range.contains(10));
    /// assert!(range.contains(20));
    ///
    /// assert!(range is std::ops::RangeFull);
    /// ```
    #[rune::function(keep)]
    pub(crate) fn contains(&self, _: Value) -> VmResult<bool> {
        VmResult::Ok(true)
    }

    /// Test the full range for partial equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = ..;
    /// assert!(range == ..);
    /// ```
    #[rune::function(keep, protocol = PARTIAL_EQ)]
    pub fn partial_eq(&self, _: &Self) -> VmResult<bool> {
        VmResult::Ok(true)
    }

    /// Test the full range for total equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::eq;
    ///
    /// let range = ..;
    /// assert!(eq(range, ..));
    /// ```
    #[rune::function(keep, protocol = EQ)]
    pub fn eq(&self, _: &Self) -> VmResult<bool> {
        VmResult::Ok(true)
    }

    /// Test the full range for partial ordering.
    ///
    /// # Examples
    ///
    /// ```rune
    /// assert!(!((..) < (..)));
    /// assert!(!((..) > (..)));
    /// ```
    #[rune::function(keep, protocol = PARTIAL_CMP)]
    pub fn partial_cmp(&self, _: &Self) -> VmResult<Option<Ordering>> {
        VmResult::Ok(Some(Ordering::Equal))
    }

    /// Test the full range for total ordering.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::cmp;
    /// use std::cmp::Ordering;
    ///
    /// assert_eq!(cmp(.., ..), Ordering::Equal);
    /// ```
    #[rune::function(keep, protocol = CMP)]
    pub fn cmp(&self, _: &Self) -> VmResult<Ordering> {
        VmResult::Ok(Ordering::Equal)
    }
}

impl fmt::Debug for RangeFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "..")
    }
}

impl ToValue for ops::RangeFull {
    fn to_value(self) -> Result<Value, RuntimeError> {
        let range = RangeFull::new();
        Ok(Value::new(range)?)
    }
}

impl FromValue for ops::RangeFull {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        let RangeFull = value.into_any::<RangeFull>()?;
        Ok(ops::RangeFull)
    }
}
