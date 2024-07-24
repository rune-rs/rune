//! Overloadable operators and associated types.

pub mod generator;

use core::cmp::Ordering;

use once_cell::sync::OnceCell;
use rune_alloc::hash_map::RandomState;

use crate as rune;
use crate::runtime::range::RangeIter;
use crate::runtime::range_from::RangeFromIter;
use crate::runtime::range_inclusive::RangeInclusiveIter;
use crate::runtime::{
    ControlFlow, EnvProtocolCaller, Function, Hasher, Range, RangeFrom, RangeFull, RangeInclusive,
    RangeTo, RangeToInclusive, Value, VmResult,
};
use crate::{ContextError, Module};

static STATE: OnceCell<RandomState> = OnceCell::new();

/// Overloadable operators and associated types.
#[rune::module(::std::ops)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;

    macro_rules! iter {
        ($ty:ident) => {
            m.ty::<$ty<u8>>()?;
            m.function_meta($ty::<u8>::next__meta)?;
            m.function_meta($ty::<u8>::size_hint__meta)?;
            m.implement_trait::<$ty<u8>>(rune::item!(::std::iter::Iterator))?;

            m.ty::<$ty<i64>>()?;
            m.function_meta($ty::<i64>::next__meta)?;
            m.function_meta($ty::<i64>::size_hint__meta)?;
            m.implement_trait::<$ty<i64>>(rune::item!(::std::iter::Iterator))?;

            m.ty::<$ty<char>>()?;
            m.function_meta($ty::<char>::next__meta)?;
            m.function_meta($ty::<char>::size_hint__meta)?;
            m.implement_trait::<$ty<char>>(rune::item!(::std::iter::Iterator))?;
        };
    }

    macro_rules! double_ended {
        ($ty:ident) => {
            iter!($ty);
            m.function_meta($ty::<u8>::next_back__meta)?;
            m.implement_trait::<$ty<u8>>(rune::item!(::std::iter::DoubleEndedIterator))?;

            m.function_meta($ty::<u8>::len__meta)?;
            m.implement_trait::<$ty<u8>>(rune::item!(::std::iter::ExactSizeIterator))?;

            m.function_meta($ty::<i64>::next_back__meta)?;
            m.implement_trait::<$ty<i64>>(rune::item!(::std::iter::DoubleEndedIterator))?;

            m.function_meta($ty::<i64>::len__meta)?;
            m.implement_trait::<$ty<i64>>(rune::item!(::std::iter::ExactSizeIterator))?;

            m.function_meta($ty::<char>::next_back__meta)?;
            m.implement_trait::<$ty<char>>(rune::item!(::std::iter::DoubleEndedIterator))?;
        };
    }

    {
        m.ty::<RangeFrom>()?;
        m.function_meta(RangeFrom::iter__meta)?;
        m.function_meta(RangeFrom::into_iter__meta)?;
        m.function_meta(RangeFrom::contains__meta)?;

        m.function_meta(RangeFrom::partial_eq__meta)?;
        m.implement_trait::<RangeFrom>(rune::item!(::std::cmp::PartialEq))?;

        m.function_meta(RangeFrom::eq__meta)?;
        m.implement_trait::<RangeFrom>(rune::item!(::std::cmp::Eq))?;

        m.function_meta(RangeFrom::partial_cmp__meta)?;
        m.implement_trait::<RangeFrom>(rune::item!(::std::cmp::PartialOrd))?;

        m.function_meta(RangeFrom::cmp__meta)?;
        m.implement_trait::<RangeFrom>(rune::item!(::std::cmp::Ord))?;

        iter!(RangeFromIter);
    }

    {
        m.ty::<RangeFull>()?;
        m.function_meta(RangeFull::contains)?;
    }

    {
        m.ty::<RangeInclusive>()?;
        m.function_meta(RangeInclusive::iter__meta)?;
        m.function_meta(RangeInclusive::into_iter__meta)?;
        m.function_meta(RangeInclusive::contains__meta)?;

        m.function_meta(RangeInclusive::partial_eq__meta)?;
        m.implement_trait::<RangeInclusive>(rune::item!(::std::cmp::PartialEq))?;

        m.function_meta(RangeInclusive::eq__meta)?;
        m.implement_trait::<RangeInclusive>(rune::item!(::std::cmp::Eq))?;

        m.function_meta(RangeInclusive::partial_cmp__meta)?;
        m.implement_trait::<RangeInclusive>(rune::item!(::std::cmp::PartialOrd))?;

        m.function_meta(RangeInclusive::cmp__meta)?;
        m.implement_trait::<RangeInclusive>(rune::item!(::std::cmp::Ord))?;

        double_ended!(RangeInclusiveIter);
    }

    {
        m.ty::<RangeToInclusive>()?;
        m.function_meta(RangeToInclusive::contains__meta)?;

        m.function_meta(RangeToInclusive::partial_eq__meta)?;
        m.implement_trait::<RangeToInclusive>(rune::item!(::std::cmp::PartialEq))?;

        m.function_meta(RangeToInclusive::eq__meta)?;
        m.implement_trait::<RangeToInclusive>(rune::item!(::std::cmp::Eq))?;

        m.function_meta(RangeToInclusive::partial_cmp__meta)?;
        m.implement_trait::<RangeToInclusive>(rune::item!(::std::cmp::PartialOrd))?;

        m.function_meta(RangeToInclusive::cmp__meta)?;
        m.implement_trait::<RangeToInclusive>(rune::item!(::std::cmp::Ord))?;
    }

    {
        m.ty::<RangeTo>()?;
        m.function_meta(RangeTo::contains__meta)?;

        m.function_meta(RangeTo::partial_eq__meta)?;
        m.implement_trait::<RangeTo>(rune::item!(::std::cmp::PartialEq))?;

        m.function_meta(RangeTo::eq__meta)?;
        m.implement_trait::<RangeTo>(rune::item!(::std::cmp::Eq))?;

        m.function_meta(RangeTo::partial_cmp__meta)?;
        m.implement_trait::<RangeTo>(rune::item!(::std::cmp::PartialOrd))?;

        m.function_meta(RangeTo::cmp__meta)?;
        m.implement_trait::<RangeTo>(rune::item!(::std::cmp::Ord))?;
    }

    {
        m.ty::<Range>()?;
        m.function_meta(Range::iter__meta)?;
        m.function_meta(Range::into_iter__meta)?;
        m.function_meta(Range::contains__meta)?;

        m.function_meta(Range::partial_eq__meta)?;
        m.implement_trait::<Range>(rune::item!(::std::cmp::PartialEq))?;

        m.function_meta(Range::eq__meta)?;
        m.implement_trait::<Range>(rune::item!(::std::cmp::Eq))?;

        m.function_meta(Range::partial_cmp__meta)?;
        m.implement_trait::<Range>(rune::item!(::std::cmp::PartialOrd))?;

        m.function_meta(Range::cmp__meta)?;
        m.implement_trait::<Range>(rune::item!(::std::cmp::Ord))?;

        double_ended!(RangeIter);
    }

    {
        m.ty::<ControlFlow>()?;
    }

    m.ty::<Function>()?;

    m.function_meta(partial_eq)?;
    m.function_meta(eq)?;
    m.function_meta(partial_cmp)?;
    m.function_meta(cmp)?;
    m.function_meta(hash)?;

    m.reexport(["Generator"], rune::item!(::std::ops::generator::Generator))?;
    m.reexport(
        ["GeneratorState"],
        rune::item!(::std::ops::generator::GeneratorState),
    )?;
    Ok(m)
}

/// Perform a partial equality check over two values.
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
/// use std::ops::partial_eq;
///
/// assert!(partial_eq(1.0, 1.0));
/// assert!(!partial_eq(1.0, 2.0));
/// ```
#[rune::function]
fn partial_eq(lhs: Value, rhs: Value) -> VmResult<bool> {
    Value::partial_eq(&lhs, &rhs)
}

/// Perform a partial equality check over two values.
///
/// This produces the same behavior as the equality operator (`==`).
///
/// For non-builtin types this leans on the behavior of the [`EQ`] protocol.
///
/// # Panics
///
/// Panics if we're trying to compare two values which are not comparable.
///
/// # Examples
///
/// ```rune
/// use std::ops::eq;
///
/// assert!(eq(1.0, 1.0));
/// assert!(!eq(1.0, 2.0));
/// ```
#[rune::function]
fn eq(lhs: Value, rhs: Value) -> VmResult<bool> {
    Value::eq(&lhs, &rhs)
}

/// Perform a partial comparison over two values.
///
/// This produces the same behavior as when comparison operators like less than
/// (`<`) is used.
///
/// For non-builtin types this leans on the behavior of the [`PARTIAL_CMP`]
/// protocol.
///
/// # Panics
///
/// Panics if we're trying to compare two values which are not comparable.
///
/// # Examples
///
/// ```rune
/// use std::ops::partial_cmp;
/// use std::cmp::Ordering;
///
/// assert_eq!(partial_cmp(1.0, 1.0), Some(Ordering::Equal));
/// assert_eq!(partial_cmp(1.0, 2.0), Some(Ordering::Less));
/// assert_eq!(partial_cmp(1.0, f64::NAN), None);
/// ```
#[rune::function]
fn partial_cmp(lhs: Value, rhs: Value) -> VmResult<Option<Ordering>> {
    Value::partial_cmp(&lhs, &rhs)
}

/// Perform a total comparison over two values.
///
/// For non-builtin types this leans on the behavior of the [`CMP`] protocol.
///
/// # Panics
///
/// Panics if we're trying to compare two values which are not comparable.
///
/// ```rune,should_panic
/// use std::ops::cmp;
///
/// let _ = cmp(1.0, f64::NAN);
/// ```
///
/// # Examples
///
/// ```rune
/// use std::ops::cmp;
/// use std::cmp::Ordering;
///
/// assert_eq!(cmp(1, 1), Ordering::Equal);
/// assert_eq!(cmp(1, 2), Ordering::Less);
/// ```
#[rune::function]
fn cmp(lhs: Value, rhs: Value) -> VmResult<Ordering> {
    Value::cmp(&lhs, &rhs)
}

/// Hashes the given value.
///
/// For non-builtin types this uses the [`HASH`] protocol.
///
/// # Hash stability
///
/// The hash is guaranteed to be stable within a single virtual machine
/// invocation, but not across virtual machines. So returning the hash from one
/// and calculating it in another using an identical value is not guaranteed to
/// produce the same hash.
///
/// # Panics
///
/// Panics if we try to generate a hash from an unhashable value.
///
/// # Examples
///
/// ```rune
/// use std::ops::hash;
///
/// assert_eq!(hash([1, 2]), hash((1, 2)));
/// ```
#[rune::function]
fn hash(value: Value) -> VmResult<i64> {
    let state = STATE.get_or_init(RandomState::new);
    let mut hasher = Hasher::new_with(state);

    vm_try!(Value::hash_with(
        &value,
        &mut hasher,
        &mut EnvProtocolCaller
    ));

    VmResult::Ok(hasher.finish() as i64)
}
