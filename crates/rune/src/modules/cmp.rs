//! Comparison and ordering.

use core::cmp::Ordering;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::runtime::{Formatter, Protocol, Value, VmResult};
use crate::shared::Caller;
use crate::{ContextError, Module};

/// Comparison and ordering.
#[rune::module(::std::cmp)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;

    {
        let ty = m.ty::<Ordering>()?.docs(docstring! {
            /// An `Ordering` is the result of a comparison between two values.
            ///
            /// # Examples
            ///
            /// ```rune
            /// use std::cmp::Ordering;
            /// use std::ops::cmp;
            ///
            /// let result = 1.cmp(2);
            /// assert_eq!(Ordering::Less, result);
            ///
            /// let result = 1.cmp(1);
            /// assert_eq!(Ordering::Equal, result);
            ///
            /// let result = 2.cmp(1);
            /// assert_eq!(Ordering::Greater, result);
            /// ```
        })?;

        let mut ty = ty.make_enum(&["Less", "Equal", "Greater"])?;

        ty.variant_mut(0)?
            .make_empty()?
            .constructor(|| Ordering::Less)?
            .docs(docstring! {
                /// "An ordering where a compared value is less than another.
            })?;

        ty.variant_mut(1)?
            .make_empty()?
            .constructor(|| Ordering::Equal)?
            .docs(docstring! {
                /// "An ordering where a compared value is equal to another.
            })?;

        ty.variant_mut(2)?
            .make_empty()?
            .constructor(|| Ordering::Greater)?
            .docs(docstring! {
                /// "An ordering where a compared value is greater than another.
            })?;
    }

    m.function_meta(ordering_partial_eq__meta)?;
    m.implement_trait::<Ordering>(rune::item!(::std::cmp::PartialEq))?;

    m.function_meta(ordering_eq__meta)?;
    m.implement_trait::<Ordering>(rune::item!(::std::cmp::Eq))?;

    m.function_meta(ordering_string_debug)?;
    m.function_meta(min)?;
    m.function_meta(max)?;

    let mut t = m.define_trait(["PartialEq"])?;

    t.docs(docstring! {
        /// Trait to perform a partial equality check over two values.
        ///
        /// This produces the same behavior as the equality operator (`==`).
        ///
        /// For non-builtin types this leans on the behavior of the [`PARTIAL_EQ`]
        /// protocol.
        ///
        /// # Panics
        ///
        /// Panics if we're trying to compare two values which are not comparable.
        ///
        /// # Examples
        ///
        /// ```rune
        /// assert!((1.0).eq(1.0));
        /// assert!(!(1.0).eq(2.0));
        ///
        /// assert!(1.0 == 1.0);
        /// assert!(1.0 != 2.0);
        /// ```
    })?;

    t.handler(|cx| {
        let partial_eq = cx.find(Protocol::PARTIAL_EQ)?;
        cx.function_handler("eq", &partial_eq)?;

        let partial_eq = Caller::<bool>::new(partial_eq);

        cx.function("ne", move |a: Value, b: Value| {
            VmResult::Ok(!vm_try!(partial_eq.call((a, b))))
        })?;

        Ok(())
    })?;

    t.function("eq")?
        .argument_types::<(Value, Value)>()?
        .return_type::<bool>()?
        .docs(docstring! {
            /// Compare two values for equality.
            ///
            /// # Examples
            ///
            /// ```rune
            /// assert_eq!(1.eq(2), false);
            /// assert_eq!(2.eq(2), true);
            /// assert_eq!(2.eq(1), false);
            /// ```
        })?;

    t.function("ne")?
        .argument_types::<(Value, Value)>()?
        .return_type::<bool>()?
        .docs(docstring! {
            /// Compare two values for inequality.
            ///
            /// # Examples
            ///
            /// ```rune
            /// assert_eq!(1.ne(2), true);
            /// assert_eq!(2.ne(2), false);
            /// assert_eq!(2.ne(1), true);
            /// ```
        })?;

    let mut t = m.define_trait(["Eq"])?;

    t.handler(|cx| {
        _ = cx.find(Protocol::EQ)?;
        Ok(())
    })?;

    t.docs(docstring! {
        /// Trait for equality comparisons.
        ///
        /// This trait allows for comparing whether two values are equal or not.
        ///
        /// # Examples
        ///
        /// ```rune
        /// use std::cmp::Eq;
        ///
        /// assert!(1.eq(1));
        /// assert!(!1.eq(2));
        /// ```
    })?;

    let mut t = m.define_trait(["PartialOrd"])?;

    t.handler(|cx| {
        let partial_cmp = cx.find(Protocol::PARTIAL_CMP)?;
        cx.function_handler("partial_cmp", &partial_cmp)?;
        Ok(())
    })?;

    t.function("partial_cmp")?
        .argument_types::<(Value, Value)>()?
        .return_type::<Option<Ordering>>()?
        .docs(docstring! {
            /// Compare two values.
            ///
            /// # Examples
            ///
            /// ```rune
            /// use std::cmp::Ordering;
            ///
            /// assert_eq!(1.partial_cmp(2), Some(Ordering::Less));
            /// assert_eq!(2.partial_cmp(2), Some(Ordering::Equal));
            /// assert_eq!(2.partial_cmp(1), Some(Ordering::Greater));
            /// ```
        })?;

    let mut t = m.define_trait(["Ord"])?;

    t.handler(|cx| {
        let cmp = cx.find(Protocol::CMP)?;
        cx.function_handler("cmp", &cmp)?;
        Ok(())
    })?;

    t.function("cmp")?
        .argument_types::<(Value, Value)>()?
        .return_type::<Ordering>()?
        .docs(docstring! {
            /// Compare two values.
            ///
            /// # Examples
            ///
            /// ```rune
            /// use std::cmp::Ordering;
            ///
            /// assert_eq!(1.cmp(2), Ordering::Less);
            /// assert_eq!(2.cmp(2), Ordering::Equal);
            /// assert_eq!(2.cmp(1), Ordering::Greater);
            /// ```
        })?;

    Ok(m)
}

/// Compares and returns the maximum of two values.
///
/// Returns the second argument if the comparison determines them to be equal.
///
/// Internally uses the [`CMP`] protocol.
///
/// # Examples
///
/// ```rune
/// use std::cmp::max;
///
/// assert_eq!(max(1, 2), 2);
/// assert_eq!(max(2, 2), 2);
/// ```
#[rune::function]
fn max(v1: Value, v2: Value) -> VmResult<Value> {
    VmResult::Ok(match vm_try!(Value::cmp(&v1, &v2)) {
        Ordering::Less | Ordering::Equal => v2,
        Ordering::Greater => v1,
    })
}

/// Compares and returns the minimum of two values.
///
/// Returns the first argument if the comparison determines them to be equal.
///
/// Internally uses the [`CMP`] protocol.
///
/// # Examples
///
/// ```rune
/// use std::cmp::min;
///
/// assert_eq!(min(1, 2), 1);
/// assert_eq!(min(2, 2), 2);
/// ```
#[rune::function]
fn min(v1: Value, v2: Value) -> VmResult<Value> {
    VmResult::Ok(match vm_try!(Value::cmp(&v1, &v2)) {
        Ordering::Less | Ordering::Equal => v1,
        Ordering::Greater => v2,
    })
}

/// Perform a partial ordering equality test.
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
///
/// assert!(Ordering::Less == Ordering::Less);
/// assert!(Ordering::Less != Ordering::Equal);
/// ```
#[rune::function(keep, instance, protocol = PARTIAL_EQ)]
fn ordering_partial_eq(this: Ordering, other: Ordering) -> bool {
    this == other
}

/// Perform a total ordering equality test.
///
/// # Examples
///
/// ```rune
/// use std::ops::eq;
/// use std::cmp::Ordering;
///
/// assert!(eq(Ordering::Less, Ordering::Less));
/// assert!(!eq(Ordering::Less, Ordering::Equal));
/// ```
#[rune::function(keep, instance, protocol = EQ)]
fn ordering_eq(this: Ordering, other: Ordering) -> bool {
    this == other
}

/// Debug format [`Ordering`].
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
///
/// assert_eq!(format!("{:?}", Ordering::Less), "Less");
/// ```
#[rune::function(instance, protocol = STRING_DEBUG)]
fn ordering_string_debug(this: Ordering, s: &mut Formatter) -> VmResult<()> {
    vm_write!(s, "{:?}", this);
    VmResult::Ok(())
}
