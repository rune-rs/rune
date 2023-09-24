use core::cmp::Ordering;
use core::fmt;
use core::ops;

use crate as rune;
use crate::runtime::{FromValue, ProtocolCaller, ToValue, Value, VmResult};
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
#[derive(Any, Default, Clone)]
#[rune(builtin, constructor, from_value = Value::into_range_full, static_type = RANGE_FULL_TYPE)]
pub struct RangeFull;

impl RangeFull {
    /// Construct a new range.
    pub const fn new() -> Self {
        Self
    }

    pub(crate) fn partial_eq_with(
        _: &Self,
        _: &Self,
        _: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        VmResult::Ok(true)
    }

    pub(crate) fn eq_with(_: &Self, _: &Self, _: &mut impl ProtocolCaller) -> VmResult<bool> {
        VmResult::Ok(true)
    }

    pub(crate) fn partial_cmp_with(
        _: &Self,
        _: &Self,
        _: &mut impl ProtocolCaller,
    ) -> VmResult<Option<Ordering>> {
        VmResult::Ok(Some(Ordering::Equal))
    }

    pub(crate) fn cmp_with(_: &Self, _: &Self, _: &mut impl ProtocolCaller) -> VmResult<Ordering> {
        VmResult::Ok(Ordering::Equal)
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
    #[rune::function]
    pub(crate) fn contains(&self, _: Value) -> VmResult<bool> {
        VmResult::Ok(true)
    }
}

impl fmt::Debug for RangeFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "..")
    }
}

impl ToValue for ops::RangeFull {
    fn to_value(self) -> VmResult<Value> {
        let range = RangeFull::new();
        VmResult::Ok(vm_try!(Value::try_from(range)))
    }
}

impl FromValue for ops::RangeFull {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        let RangeFull = vm_try!(vm_try!(value.into_range_full()).take());
        VmResult::Ok(ops::RangeFull)
    }
}
