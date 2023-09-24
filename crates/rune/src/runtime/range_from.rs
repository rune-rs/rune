use core::cmp::Ordering;
use core::fmt;
use core::ops;

use crate as rune;
use crate::runtime::{
    EnvProtocolCaller, FromValue, Iterator, ProtocolCaller, ToValue, Value, VmErrorKind, VmResult,
};
use crate::Any;

/// Type for a from range expression `start..`.
///
/// # Examples
///
/// ```rune
/// let range = 0..;
///
/// assert!(!range.contains(-10));
/// assert!(range.contains(5));
/// assert!(range.contains(10));
/// assert!(range.contains(20));
///
/// assert!(range is std::ops::RangeFrom);
/// ```
///
/// Ranges can contain any type:
///
/// ```rune
/// let range = 'a'..;
/// assert_eq!(range.start, 'a');
/// range.start = 'b';
/// assert_eq!(range.start, 'b');
/// ```
///
/// Certain ranges can be used as iterators:
///
/// ```rune
/// let range = 'a'..;
/// assert_eq!(range.iter().take(5).collect::<Vec>(), ['a', 'b', 'c', 'd', 'e']);
/// ```
///
/// # Rust examples
///
/// ```rust
/// use rune::runtime::RangeFrom;
///
/// let start = rune::to_value(1)?;
/// let _ = RangeFrom::new(start);
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Any, Clone)]
#[rune(builtin, constructor, from_value = Value::into_range_from, static_type = RANGE_FROM_TYPE)]
pub struct RangeFrom {
    /// The start value of the range.
    #[rune(get, set)]
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

    /// Build an iterator over the range.
    ///
    /// # Panics
    ///
    /// This panics if the range is not a well-defined range.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let vec = [];
    ///
    /// for value in 'a'.. {
    ///     vec.push(value);
    ///
    ///     if value == 'e' {
    ///        break;
    ///     }
    /// }
    ///
    /// assert_eq!(vec, ['a', 'b', 'c', 'd', 'e']);
    /// ```
    ///
    /// Cannot construct an iterator over floats:
    ///
    /// ```rune,should_panic
    /// let range = 1.0..;
    ///
    /// for value in 1.0 .. {
    /// }
    /// ```
    #[rune::function(keep, protocol = INTO_ITER)]
    pub fn into_iter(&self) -> VmResult<Iterator> {
        self.iter()
    }

    /// Test the range for partial equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = 'a'..;
    /// assert!(range == ('a'..));
    /// assert!(range != ('b'..));
    ///
    /// let range = 1.0..;
    /// assert!(range == (1.0..));
    /// assert!(range != (f64::NAN..));
    /// assert!((f64::NAN..) != (f64::NAN..));
    /// ```
    #[rune::function(keep, protocol = PARTIAL_EQ)]
    pub fn partial_eq(&self, other: &Self) -> VmResult<bool> {
        self.partial_eq_with(other, &mut EnvProtocolCaller)
    }

    pub(crate) fn partial_eq_with(
        &self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        Value::partial_eq_with(&self.start, &b.start, caller)
    }

    /// Test the range for total equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::eq;
    ///
    /// let range = 'a'..;
    /// assert!(eq(range, 'a'..));
    /// assert!(!eq(range, 'b'..));
    /// ```
    #[rune::function(keep, protocol = EQ)]
    pub fn eq(&self, other: &Self) -> VmResult<bool> {
        self.eq_with(other, &mut EnvProtocolCaller)
    }

    pub(crate) fn eq_with(&self, b: &Self, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        Value::eq_with(&self.start, &b.start, caller)
    }

    /// Test the range for partial ordering.
    ///
    /// # Examples
    ///
    /// ```rune
    /// assert!(('a'..) < ('b'..));
    /// assert!(('c'..) > ('b'..));
    /// assert!(!((f64::NAN..) > (f64::INFINITY..)));
    /// assert!(!((f64::NAN..) < (f64::INFINITY..)));
    /// ```
    #[rune::function(keep, protocol = PARTIAL_CMP)]
    pub fn partial_cmp(&self, other: &Self) -> VmResult<Option<Ordering>> {
        self.partial_cmp_with(other, &mut EnvProtocolCaller)
    }

    pub(crate) fn partial_cmp_with(
        &self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Option<Ordering>> {
        Value::partial_cmp_with(&self.start, &b.start, caller)
    }

    /// Test the range for total ordering.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::cmp;
    /// use std::cmp::Ordering;
    ///
    /// assert_eq!(cmp('a'.., 'b'..), Ordering::Less);
    /// assert_eq!(cmp('c'.., 'b'..), Ordering::Greater);
    /// ```
    #[rune::function(keep, protocol = CMP)]
    pub fn cmp(&self, other: &Self) -> VmResult<Ordering> {
        self.cmp_with(other, &mut EnvProtocolCaller)
    }

    pub(crate) fn cmp_with(
        &self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Ordering> {
        Value::cmp_with(&self.start, &b.start, caller)
    }

    /// Test if the range contains the given value.
    ///
    /// The check is performed using the [`PARTIAL_CMP`] protocol.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = 0..;
    ///
    /// assert!(!range.contains(-10));
    /// assert!(range.contains(5));
    /// assert!(range.contains(10));
    /// assert!(range.contains(20));
    ///
    /// assert!(range is std::ops::RangeFrom);
    /// ```
    #[rune::function(keep)]
    pub(crate) fn contains(&self, value: Value) -> VmResult<bool> {
        self.contains_with(value, &mut EnvProtocolCaller)
    }

    pub(crate) fn contains_with(
        &self,
        value: Value,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        VmResult::Ok(matches!(
            vm_try!(Value::partial_cmp_with(&self.start, &value, caller)),
            Some(Ordering::Less | Ordering::Equal)
        ))
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
        VmResult::Ok(vm_try!(Value::try_from(range)))
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
