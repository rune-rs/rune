use core::cmp::Ordering;
use core::fmt;
use core::ops;

use crate as rune;
use crate::compile::Named;
use crate::module::InstallWith;
use crate::runtime::{
    EnvProtocolCaller, FromValue, Iterator, ProtocolCaller, RawStr, ToValue, Value, VmErrorKind,
    VmResult,
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

    /// Iterate over the range.
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
    /// for value in 'a'..'e' {
    ///     vec.push(value);
    /// }
    ///
    /// assert_eq!(vec, ['a', 'b', 'c', 'd']);
    /// ```
    ///
    /// Cannot construct an iterator over floats:
    ///
    /// ```rune,should_panic
    /// for value in 1.0..2.0 {
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
    /// let range = 'a'..'e';
    /// assert!(range == ('a'..'e'));
    /// assert!(range != ('b'..'e'));
    ///
    /// let range = 1.0..2.0;
    /// assert!(range == (1.0..2.0));
    /// assert!(range != (f64::NAN..2.0));
    /// assert!((f64::NAN..2.0) != (f64::NAN..2.0));
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
        if !vm_try!(Value::partial_eq_with(&self.start, &b.start, caller)) {
            return VmResult::Ok(false);
        }

        Value::partial_eq_with(&self.end, &b.end, caller)
    }

    /// Test the range for total equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::eq;
    ///
    /// let range = 'a'..'e';
    /// assert!(eq(range, 'a'..'e'));
    /// assert!(!eq(range, 'b'..'e'));
    /// ```
    #[rune::function(keep, protocol = EQ)]
    pub fn eq(&self, other: &Self) -> VmResult<bool> {
        self.eq_with(other, &mut EnvProtocolCaller)
    }

    pub(crate) fn eq_with(&self, b: &Self, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        if !vm_try!(Value::eq_with(&self.start, &b.start, caller)) {
            return VmResult::Ok(false);
        }

        Value::eq_with(&self.end, &b.end, caller)
    }

    /// Test the range for partial ordering.
    ///
    /// # Examples
    ///
    /// ```rune
    /// assert!(('a'..'e') < ('b'..'e'));
    /// assert!(('c'..'e') > ('b'..'e'));
    /// assert!(!((f64::NAN..2.0) > (f64::INFINITY..2.0)));
    /// assert!(!((f64::NAN..2.0) < (f64::INFINITY..2.0)));
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
        match vm_try!(Value::partial_cmp_with(&self.start, &b.start, caller)) {
            Some(Ordering::Equal) => (),
            other => return VmResult::Ok(other),
        }

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
    /// assert_eq!(cmp('a'..'e', 'b'..'e'), Ordering::Less);
    /// assert_eq!(cmp('c'..'e', 'b'..'e'), Ordering::Greater);
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
        match vm_try!(Value::cmp_with(&self.start, &b.start, caller)) {
            Ordering::Equal => (),
            other => return VmResult::Ok(other),
        }

        Value::cmp_with(&self.end, &b.end, caller)
    }

    /// Test if the range contains the given value.
    ///
    /// The check is performed using the [`PARTIAL_CMP`] protocol.
    ///
    /// # Examples
    ///
    /// ```rune
    /// let range = 0..10;
    ///
    /// assert!(!range.contains(-10));
    /// assert!(range.contains(5));
    /// assert!(!range.contains(10));
    /// assert!(!range.contains(20));
    ///
    /// assert!(range is std::ops::Range);
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
        match vm_try!(Value::partial_cmp_with(&self.start, &value, caller)) {
            Some(Ordering::Less | Ordering::Equal) => {}
            _ => return VmResult::Ok(false),
        }

        VmResult::Ok(matches!(
            vm_try!(Value::partial_cmp_with(&self.end, &value, caller)),
            Some(Ordering::Greater)
        ))
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
