use core::cmp::Ordering;
use core::fmt;
use core::ops;

use crate as rune;
use crate::alloc::clone::TryClone;
use crate::runtime::{
    EnvProtocolCaller, FromValue, Inline, ProtocolCaller, RefRepr, RuntimeError, ToValue, Value,
    VmErrorKind, VmResult,
};
use crate::Any;

use super::StepsBetween;

/// Type for a range expression `start..end`.
///
/// # Examples
///
/// ```rune
/// let range = 0..10;
/// assert!(!range.contains(-10));
/// assert!(range.contains(5));
/// assert!(!range.contains(10));
/// assert!(!range.contains(20));
///
/// assert!(range is std::ops::Range);
/// ```
///
/// Ranges can contain any type:
///
/// ```rune
/// let range = 'a'..'f';
/// assert_eq!(range.start, 'a');
/// range.start = 'b';
/// assert_eq!(range.start, 'b');
/// assert_eq!(range.end, 'f');
/// range.end = 'g';
/// assert_eq!(range.end, 'g');
/// ```
///
/// Certain ranges can be used as iterators:
///
/// ```rune
/// let range = 'a'..'e';
/// assert_eq!(range.iter().collect::<Vec>(), ['a', 'b', 'c', 'd']);
/// ```
///
/// # Examples
///
/// ```rust
/// use rune::runtime::Range;
///
/// let start = rune::to_value(1)?;
/// let end = rune::to_value(10)?;
/// let _ = Range::new(start, end);
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Any, Clone, TryClone)]
#[try_clone(crate)]
#[rune(constructor, static_type = RANGE)]
pub struct Range {
    /// The start value of the range.
    #[rune(get, set)]
    pub start: Value,
    /// The to value of the range.
    #[rune(get, set)]
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
    pub fn iter(&self) -> VmResult<Value> {
        let value = match (
            &vm_try!(self.start.as_ref_repr()),
            vm_try!(self.end.as_ref_repr()),
        ) {
            (RefRepr::Inline(Inline::Byte(start)), RefRepr::Inline(end)) => {
                let end = vm_try!(end.try_as_integer::<u8>());
                vm_try!(rune::to_value(RangeIter::new(*start..end)))
            }
            (RefRepr::Inline(Inline::Unsigned(start)), RefRepr::Inline(end)) => {
                let end = vm_try!(end.try_as_integer::<u64>());
                vm_try!(rune::to_value(RangeIter::new(*start..end)))
            }
            (RefRepr::Inline(Inline::Signed(start)), RefRepr::Inline(end)) => {
                let end = vm_try!(end.try_as_integer::<i64>());
                vm_try!(rune::to_value(RangeIter::new(*start..end)))
            }
            (RefRepr::Inline(Inline::Char(start)), RefRepr::Inline(Inline::Char(end))) => {
                vm_try!(rune::to_value(RangeIter::new(*start..*end)))
            }
            (start, end) => {
                return VmResult::err(VmErrorKind::UnsupportedIterRange {
                    start: vm_try!(start.type_info()),
                    end: vm_try!(end.type_info()),
                })
            }
        };

        VmResult::Ok(value)
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
    pub fn into_iter(&self) -> VmResult<Value> {
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
        caller: &mut dyn ProtocolCaller,
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

    pub(crate) fn eq_with(&self, b: &Self, caller: &mut dyn ProtocolCaller) -> VmResult<bool> {
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
        caller: &mut dyn ProtocolCaller,
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

    pub(crate) fn cmp_with(&self, b: &Self, caller: &mut dyn ProtocolCaller) -> VmResult<Ordering> {
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
        caller: &mut dyn ProtocolCaller,
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
    fn to_value(self) -> Result<Value, RuntimeError> {
        let start = self.start.to_value()?;
        let end = self.end.to_value()?;
        let range = Range::new(start, end);
        Ok(Value::new(range)?)
    }
}

impl<Idx> FromValue for ops::Range<Idx>
where
    Idx: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        let range = value.into_any::<Range>()?;
        let start = Idx::from_value(range.start)?;
        let end = Idx::from_value(range.end)?;
        Ok(ops::Range { start, end })
    }
}

double_ended_range_iter!(Range, RangeIter<T>, {
    #[rune::function(instance, keep, protocol = SIZE_HINT)]
    #[inline]
    pub(crate) fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[rune::function(instance, keep, protocol = LEN)]
    #[inline]
    pub(crate) fn len(&self) -> VmResult<usize>
    where
        T: Copy + StepsBetween + fmt::Debug,
    {
        let Some(result) = T::steps_between(self.iter.start, self.iter.end) else {
            return VmResult::panic(format!(
                "could not calculate length of range {:?}..={:?}",
                self.iter.start, self.iter.end
            ));
        };

        VmResult::Ok(result)
    }
});
