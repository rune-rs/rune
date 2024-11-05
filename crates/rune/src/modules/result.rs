//! The [`Result`] type.

use core::cmp::Ordering;
use core::hash::Hasher as _;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::runtime::{
    ControlFlow, EnvProtocolCaller, Formatter, Function, Hasher, Panic, Protocol, Value, VmResult,
};
use crate::{ContextError, Module};

/// The [`Result`] type.
///
/// This module deals with the fundamental [`Result`] type in Rune.
#[rune::module(::std::result)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;

    // Sorted for ease of finding
    let mut result = m
        .ty::<Result<Value, Value>>()?
        .static_docs(&["Result is a type that represents either success (Ok) or failure (Err)."])?
        .make_enum(&["Ok", "Err"])?;

    result
        .variant_mut(0)?
        .make_unnamed(1)?
        .constructor(Result::Ok)?
        .static_docs(&["Contains the success value"])?;

    result
        .variant_mut(1)?
        .make_unnamed(1)?
        .constructor(Result::Err)?
        .static_docs(&["Contains the error value"])?;

    m.associated_function(
        &Protocol::IS_VARIANT,
        |this: &Result<Value, Value>, index: usize| match (this, index) {
            (Result::Ok(_), 0) => true,
            (Result::Err(_), 1) => true,
            _ => false,
        },
    )?;

    m.index_function(
        &Protocol::GET,
        0,
        |this: &Result<Value, Value>| match this {
            Result::Ok(value) => VmResult::Ok(value.clone()),
            Result::Err(value) => VmResult::Ok(value.clone()),
        },
    )?;

    m.function_meta(ok)?;
    m.function_meta(is_ok)?;
    m.function_meta(is_err)?;
    m.function_meta(unwrap)?;
    m.function_meta(unwrap_or)?;
    m.function_meta(unwrap_or_else)?;
    m.function_meta(expect)?;
    m.function_meta(and_then)?;
    m.function_meta(map)?;

    m.function_meta(clone__meta)?;
    m.implement_trait::<Result<Value, Value>>(rune::item!(::std::clone::Clone))?;

    m.function_meta(partial_eq__meta)?;
    m.implement_trait::<Result<Value, Value>>(rune::item!(::std::cmp::PartialEq))?;

    m.function_meta(eq__meta)?;
    m.implement_trait::<Result<Value, Value>>(rune::item!(::std::cmp::Eq))?;

    m.function_meta(partial_cmp__meta)?;
    m.implement_trait::<Result<Value, Value>>(rune::item!(::std::cmp::PartialOrd))?;

    m.function_meta(cmp__meta)?;
    m.implement_trait::<Result<Value, Value>>(rune::item!(::std::cmp::Ord))?;

    m.function_meta(hash__meta)?;
    m.function_meta(debug_fmt__meta)?;

    m.function_meta(result_try__meta)?;
    Ok(m)
}

/// Converts from `Result` to `Option`.
///
/// # Examples
///
/// ```rune
/// let a = Ok(2);
/// let b = Err(3);
///
/// assert_eq!(a.ok(), Some(2));
/// assert_eq!(b.ok(), None);
/// ```
#[rune::function(instance)]
fn ok(result: &Result<Value, Value>) -> Option<Value> {
    result.as_ref().ok().cloned()
}

/// Returns `true` if the result is [`Ok`].
///
/// # Examples
///
/// ```rune
/// let x = Ok(-3);
/// assert_eq!(x.is_ok(), true);
///
/// let x = Err("Some error message");
/// assert_eq!(x.is_ok(), false);
/// ```
#[rune::function(instance)]
fn is_ok(result: &Result<Value, Value>) -> bool {
    result.is_ok()
}

/// Returns `true` if the result is [`Err`].
///
/// # Examples
///
/// ```rune
/// let x = Ok(-3);
/// assert_eq!(x.is_err(), false);
///
/// let x = Err("Some error message");
/// assert_eq!(x.is_err(), true);
/// ```
#[rune::function(instance)]
fn is_err(result: &Result<Value, Value>) -> bool {
    result.is_err()
}

/// Returns the contained [`Ok`] value, consuming the `self` value.
///
/// Because this function may panic, its use is generally discouraged. Instead,
/// prefer to use pattern matching and handle the [`Err`] case explicitly, or
/// call [`unwrap_or`], [`unwrap_or_else`], or [`unwrap_or_default`].
///
/// [`unwrap_or`]: Result::unwrap_or
/// [`unwrap_or_else`]: Result::unwrap_or_else
/// [`unwrap_or_default`]: Result::unwrap_or_default
///
/// # Panics
///
/// Panics if the value is an [`Err`], with a panic message provided by the
/// [`Err`]'s value.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let x = Ok(2);
/// assert_eq!(x.unwrap(), 2);
/// ```
///
/// ```rune,should_panic
/// let x = Err("emergency failure");
/// x.unwrap(); // panics with `emergency failure`
/// ```
#[rune::function(instance)]
fn unwrap(result: &Result<Value, Value>) -> VmResult<Value> {
    match result {
        Ok(value) => VmResult::Ok(value.clone()),
        Err(err) => {
            let mut m = String::new();
            vm_try!(vm_write!(
                m,
                "Called `Result::unwrap()` on an `Err` value: "
            ));
            vm_try!(Formatter::format_with(&mut m, |f| err.debug_fmt(f)));
            VmResult::err(Panic::custom(m))
        }
    }
}

/// Returns the contained [`Ok`] value or a provided default.
///
/// Arguments passed to `unwrap_or` are eagerly evaluated; if you are passing
/// the result of a function call, it is recommended to use [`unwrap_or_else`],
/// which is lazily evaluated.
///
/// [`unwrap_or_else`]: Result::unwrap_or_else
///
/// # Examples
///
/// ```rune
/// let default_value = 2;
/// let x = Ok(9);
/// assert_eq!(x.unwrap_or(default_value), 9);
///
/// let x = Err("error");
/// assert_eq!(x.unwrap_or(default_value), default_value);
/// ```
#[rune::function(instance)]
fn unwrap_or(this: &Result<Value, Value>, default: Value) -> Value {
    match this {
        Ok(value) => value.clone(),
        Err(_) => default.clone(),
    }
}

/// Returns the contained [`Ok`] value or computes it from a closure.
///
///
/// # Examples
///
/// ```rune
/// fn count(x) {
///     x.len()
/// }
///
/// assert_eq!(Ok(2).unwrap_or_else(count), 2);
/// assert_eq!(Err("foo").unwrap_or_else(count), 3);
/// ```
#[rune::function(instance)]
fn unwrap_or_else(this: &Result<Value, Value>, default: Function) -> VmResult<Value> {
    match this {
        Ok(value) => VmResult::Ok(value.clone()),
        Err(error) => default.call((error,)),
    }
}

/// Returns the contained [`Ok`] value, consuming the `self` value.
///
/// Because this function may panic, its use is generally discouraged. Instead,
/// prefer to use pattern matching and handle the [`Err`] case explicitly, or
/// call [`unwrap_or`], [`unwrap_or_else`], or [`unwrap_or_default`].
///
/// [`unwrap_or`]: Result::unwrap_or
/// [`unwrap_or_else`]: Result::unwrap_or_else
/// [`unwrap_or_default`]: Result::unwrap_or_default
///
/// # Panics
///
/// Panics if the value is an [`Err`], with a panic message including the passed
/// message, and the content of the [`Err`].
///
/// # Examples
///
/// ```rune,should_panic
/// let x = Err("emergency failure");
/// x.expect("Testing expect"); // panics with `Testing expect: emergency failure`
/// ```
///
/// # Recommended Message Style
///
/// We recommend that `expect` messages are used to describe the reason you
/// _expect_ the `Result` should be `Ok`. If you're having trouble remembering
/// how to phrase expect error messages remember to focus on the word "should"
/// as in "env variable should be set by blah" or "the given binary should be
/// available and executable by the current user".
#[rune::function(instance)]
fn expect(result: &Result<Value, Value>, message: Value) -> VmResult<Value> {
    match result {
        Ok(value) => VmResult::Ok(value.clone()),
        Err(err) => {
            let mut s = String::new();
            vm_try!(Formatter::format_with(&mut s, |f| {
                vm_try!(message.display_fmt(f));
                vm_try!(f.try_write_str(": "));
                vm_try!(err.debug_fmt(f));
                VmResult::Ok(())
            }));
            VmResult::err(Panic::custom(s))
        }
    }
}

/// Calls `op` if the result is [`Ok`], otherwise returns the [`Err`] value of `self`.
///
/// This function can be used for control flow based on `Result` values.
///
/// # Examples
///
/// ```rune
/// fn sq_then_to_string(x) {
///     x.checked_mul(x).ok_or("overflowed")
/// }
///
/// assert_eq!(Ok(2).and_then(sq_then_to_string), Ok(4));
/// assert_eq!(Ok(u64::MAX).and_then(sq_then_to_string), Err("overflowed"));
/// assert_eq!(Err("not a number").and_then(sq_then_to_string), Err("not a number"));
/// ```
#[rune::function(instance)]
fn and_then(this: &Result<Value, Value>, op: Function) -> VmResult<Result<Value, Value>> {
    match this {
        Ok(v) => VmResult::Ok(vm_try!(op.call((v,)))),
        Err(e) => VmResult::Ok(Err(e.clone())),
    }
}

/// Maps a `Result<T, E>` to `Result<U, E>` by applying a function to a
/// contained [`Ok`] value, leaving an [`Err`] value untouched.
///
/// This function can be used to compose the results of two functions.
///
/// # Examples
///
/// Print the numbers on each line of a string multiplied by two.
///
/// ```rune
/// let lines = ["1", "2", "3", "4"];
/// let out = [];
///
/// for num in lines {
///     out.push(i64::parse(num).map(|i| i * 2)?);
/// }
///
/// assert_eq!(out, [2, 4, 6, 8]);
/// ```
#[rune::function(instance)]
fn map(this: &Result<Value, Value>, then: Function) -> VmResult<Result<Value, Value>> {
    match this {
        Ok(v) => VmResult::Ok(Ok(vm_try!(then.call((v,))))),
        Err(e) => VmResult::Ok(Err(e.clone())),
    }
}

/// Clone the result.
///
/// # Examples
///
/// ```rune
/// let a = Ok(b"hello world");
/// let b = a.clone();
///
/// a?.extend(b"!");
///
/// assert_eq!(a, Ok(b"hello world!"));
/// assert_eq!(b, Ok(b"hello world"));
/// ```
#[rune::function(keep, instance, protocol = CLONE)]
fn clone(this: &Result<Value, Value>) -> VmResult<Result<Value, Value>> {
    VmResult::Ok(match this {
        Ok(ok) => Ok(vm_try!(ok.clone_with(&mut EnvProtocolCaller))),
        Err(err) => Err(vm_try!(err.clone_with(&mut EnvProtocolCaller))),
    })
}

/// Test two results for partial equality.
///
/// # Examples
///
/// ```rune
/// assert_eq!(Ok(b"a") == Ok(b"a"), true);
/// assert_eq!(Ok(b"a") == Ok(b"ab"), false);
/// assert_eq!(Ok(b"ab") == Ok(b"a"), false);
/// ```
///
/// Using explicit functions:
///
/// ```rune
/// use std::ops::partial_eq;
///
/// assert_eq!(partial_eq(Ok(b"a"), Ok(b"a")), true);
/// assert_eq!(partial_eq(Ok(b"a"), Ok(b"ab")), false);
/// assert_eq!(partial_eq(Ok(b"ab"), Ok(b"a")), false);
/// ```
#[rune::function(keep, instance, protocol = PARTIAL_EQ)]
#[inline]
fn partial_eq(this: &Result<Value, Value>, rhs: &Result<Value, Value>) -> VmResult<bool> {
    match (this, rhs) {
        (Ok(a), Ok(b)) => Value::partial_eq(a, b),
        (Err(a), Err(b)) => Value::partial_eq(a, b),
        _ => VmResult::Ok(false),
    }
}

/// Test two results for total equality.
///
/// # Examples
///
/// ```rune
/// use std::ops::eq;
///
/// assert_eq!(eq(Ok(b"a"), Ok(b"a")), true);
/// assert_eq!(eq(Ok(b"a"), Ok(b"ab")), false);
/// assert_eq!(eq(Ok(b"ab"), Ok(b"a")), false);
/// ```
#[rune::function(keep, instance, protocol = EQ)]
#[inline]
fn eq(this: &Result<Value, Value>, rhs: &Result<Value, Value>) -> VmResult<bool> {
    match (this, rhs) {
        (Ok(a), Ok(b)) => Value::eq(a, b),
        (Err(a), Err(b)) => Value::eq(a, b),
        _ => VmResult::Ok(false),
    }
}

/// Perform a partial ordered comparison between two results.
///
/// # Examples
///
/// ```rune
/// assert!(Ok(b"a") < Ok(b"ab"));
/// assert!(Ok(b"ab") > Ok(b"a"));
/// assert!(Ok(b"a") == Ok(b"a"));
/// ```
///
/// Using explicit functions:
///
/// ```rune
/// use std::cmp::Ordering;
/// use std::ops::partial_cmp;
///
/// assert_eq!(partial_cmp(Ok(b"a"), Ok(b"ab")), Some(Ordering::Less));
/// assert_eq!(partial_cmp(Ok(b"ab"), Ok(b"a")), Some(Ordering::Greater));
/// assert_eq!(partial_cmp(Ok(b"a"), Ok(b"a")), Some(Ordering::Equal));
/// ```
#[rune::function(keep, instance, protocol = PARTIAL_CMP)]
#[inline]
fn partial_cmp(
    this: &Result<Value, Value>,
    rhs: &Result<Value, Value>,
) -> VmResult<Option<Ordering>> {
    match (this, rhs) {
        (Ok(a), Ok(b)) => Value::partial_cmp(a, b),
        (Err(a), Err(b)) => Value::partial_cmp(a, b),
        (Ok(..), Err(..)) => VmResult::Ok(Some(Ordering::Greater)),
        (Err(..), Ok(..)) => VmResult::Ok(Some(Ordering::Less)),
    }
}

/// Perform a totally ordered comparison between two results.
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
/// use std::ops::cmp;
///
/// assert_eq!(cmp(Ok(b"a"), Ok(b"ab")), Ordering::Less);
/// assert_eq!(cmp(Ok(b"ab"), Ok(b"a")), Ordering::Greater);
/// assert_eq!(cmp(Ok(b"a"), Ok(b"a")), Ordering::Equal);
/// ```
#[rune::function(keep, instance, protocol = CMP)]
#[inline]
fn cmp(this: &Result<Value, Value>, rhs: &Result<Value, Value>) -> VmResult<Ordering> {
    match (this, rhs) {
        (Ok(a), Ok(b)) => Value::cmp(a, b),
        (Err(a), Err(b)) => Value::cmp(a, b),
        (Ok(..), Err(..)) => VmResult::Ok(Ordering::Greater),
        (Err(..), Ok(..)) => VmResult::Ok(Ordering::Less),
    }
}

/// Hash the result.
///
/// # Examples
///
/// ```rune
/// use std::ops::hash;
///
/// let a = Ok("hello");
/// let b = Ok("hello");
///
/// assert_eq!(hash(a), hash(b));
/// ```
#[rune::function(keep, instance, protocol = HASH)]
fn hash(this: &Result<Value, Value>, hasher: &mut Hasher) -> VmResult<()> {
    match this {
        Ok(value) => {
            hasher.write_u64(0);
            vm_try!(value.hash(hasher));
        }
        Err(value) => {
            hasher.write_u64(1);
            vm_try!(value.hash(hasher));
        }
    }

    VmResult::Ok(())
}

/// Write a debug representation of a result.
///
/// # Examples
///
/// ```rune
/// println!("{:?}", Ok("Hello"));
/// println!("{:?}", Err("Hello"));
/// ```
#[rune::function(keep, instance, protocol = DEBUG_FMT)]
#[inline]
fn debug_fmt(this: &Result<Value, Value>, f: &mut Formatter) -> VmResult<()> {
    match this {
        Ok(value) => {
            vm_try!(f.try_write_str("Ok("));
            vm_try!(value.debug_fmt(f));
            vm_try!(f.try_write_str(")"));
        }
        Err(value) => {
            vm_try!(f.try_write_str("Err("));
            vm_try!(value.debug_fmt(f));
            vm_try!(f.try_write_str(")"));
        }
    }

    VmResult::Ok(())
}

/// Using [`Result`] with the try protocol.
///
/// # Examples
///
/// ```rune
/// fn maybe_add_one(value) {
///     Ok(value? + 1)
/// }
///
/// assert_eq!(maybe_add_one(Ok(4)), Ok(5));
/// assert_eq!(maybe_add_one(Err("not a number")), Err("not a number"));
/// ```
#[rune::function(keep, instance, protocol = TRY)]
pub(crate) fn result_try(this: &Result<Value, Value>) -> VmResult<ControlFlow> {
    VmResult::Ok(match this {
        Ok(value) => ControlFlow::Continue(value.clone()),
        Err(error) => ControlFlow::Break(vm_try!(Value::try_from(Err(error.clone())))),
    })
}
