//! The `std::result` module.

use crate as rune;
use crate::runtime::{Function, Panic, Value, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::result` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["result"]);
    // Sorted for ease of finding
    let mut result = module
        .result(["Result"])?
        .static_docs(&["Result is a type that represents either success (Ok) or failure (Err)."]);

    result
        .variant_mut(0)?
        .static_docs(&["Contains the success value"]);

    result
        .variant_mut(1)?
        .static_docs(&["Contains the error value"]);

    module.function_meta(ok)?;
    module.associated_function("is_ok", is_ok)?;
    module.associated_function("is_err", is_err)?;
    module.associated_function("unwrap", unwrap_impl)?;
    module.associated_function("unwrap_or", Result::<Value, Value>::unwrap_or)?;
    module.function_meta(expect)?;
    module.function_meta(and_then)?;
    module.function_meta(map)?;
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

fn is_ok(result: &Result<Value, Value>) -> bool {
    result.is_ok()
}

fn is_err(result: &Result<Value, Value>) -> bool {
    result.is_err()
}

fn unwrap_impl(result: Result<Value, Value>) -> VmResult<Value> {
    match result {
        Ok(value) => VmResult::Ok(value),
        Err(err) => VmResult::err(Panic::msg(format_args!(
            "called `Result::unwrap()` on an `Err` value: {:?}",
            err
        ))),
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
fn expect(result: Result<Value, Value>, message: &str) -> VmResult<Value> {
    match result {
        Ok(value) => VmResult::Ok(value),
        Err(err) => VmResult::err(Panic::msg(format_args!("{}: {:?}", message, err))),
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
