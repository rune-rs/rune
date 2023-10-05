//! The `std::result` module.

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::runtime::{ControlFlow, Formatter, Function, Panic, Shared, Value, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::result` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["result"])?;
    // Sorted for ease of finding
    let mut result = module
        .result("Result")?
        .static_docs(&["Result is a type that represents either success (Ok) or failure (Err)."])?;

    result
        .variant_mut(0)?
        .static_docs(&["Contains the success value"])?;

    result
        .variant_mut(1)?
        .static_docs(&["Contains the error value"])?;

    module.function_meta(ok)?;
    module.function_meta(is_ok)?;
    module.function_meta(is_err)?;
    module.function_meta(unwrap)?;
    module.function_meta(unwrap_or)?;
    module.function_meta(unwrap_or_else)?;
    module.function_meta(expect)?;
    module.function_meta(and_then)?;
    module.function_meta(map)?;
    module.function_meta(result_try__meta)?;
    Ok(module)
}

/// Converts from `Result<T, E>` to `Option<T>`.
///
/// Converts self into an `Option<T>`, consuming `self`, and discarding the
/// error, if any.
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
fn unwrap(result: Result<Value, Value>) -> VmResult<Value> {
    match result {
        Ok(value) => VmResult::Ok(value),
        Err(err) => {
            let message = vm_try!(format_args!(
                "Called `Result::unwrap()` on an `Err` value: {:?}",
                err
            )
            .try_to_string());
            VmResult::err(Panic::custom(message))
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
fn unwrap_or(this: Result<Value, Value>, default: Value) -> Value {
    this.unwrap_or(default)
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
fn unwrap_or_else(this: Result<Value, Value>, default: Function) -> VmResult<Value> {
    match this {
        Ok(value) => VmResult::Ok(value),
        Err(error) => default.call::<_, Value>((error,)),
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
fn expect(result: Result<Value, Value>, message: Value) -> VmResult<Value> {
    match result {
        Ok(value) => VmResult::Ok(value),
        Err(err) => {
            let mut f = Formatter::new();
            vm_try!(message.string_display(&mut f));
            vm_try!(f.try_write_str(": "));
            vm_try!(err.string_debug(&mut f));
            VmResult::err(Panic::custom(f.into_string()))
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
/// assert_eq!(Ok(i64::MAX).and_then(sq_then_to_string), Err("overflowed"));
/// assert_eq!(Err("not a number").and_then(sq_then_to_string), Err("not a number"));
/// ```
#[rune::function(instance)]
fn and_then(this: &Result<Value, Value>, op: Function) -> VmResult<Result<Value, Value>> {
    match this {
        Ok(v) => VmResult::Ok(vm_try!(op.call::<_, _>((v,)))),
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
        Ok(v) => VmResult::Ok(Ok(vm_try!(then.call::<_, _>((v,))))),
        Err(e) => VmResult::Ok(Err(e.clone())),
    }
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
pub(crate) fn result_try(this: Result<Value, Value>) -> VmResult<ControlFlow> {
    VmResult::Ok(match this {
        Ok(value) => ControlFlow::Continue(value),
        Err(error) => ControlFlow::Break(Value::Result(vm_try!(Shared::new(Err(error))))),
    })
}
