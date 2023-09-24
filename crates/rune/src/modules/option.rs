//! The `std::option` module.

use crate as rune;
use crate::runtime::{ControlFlow, Formatter, Function, Iterator, Panic, Shared, Value, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::option` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["option"])?;
    module.option(["Option"])?;
    // Sorted for ease of finding
    module.function_meta(expect)?;
    module.function_meta(unwrap)?;
    module.function_meta(unwrap_or)?;
    module.function_meta(unwrap_or_else)?;
    module.function_meta(is_some)?;
    module.function_meta(is_none)?;
    module.function_meta(iter)?;
    module.function_meta(and_then)?;
    module.function_meta(map)?;
    module.function_meta(take)?;
    module.function_meta(transpose)?;
    module.function_meta(ok_or)?;
    module.function_meta(ok_or_else)?;
    module.function_meta(into_iter)?;
    module.function_meta(option_try__meta)?;
    Ok(module)
}

/// Returns the contained [`Some`] value, consuming the `self` value.
///
/// # Panics
///
/// Panics if the value is a [`None`] with a custom panic message provided by
/// `msg`.
///
/// # Examples
///
/// ```rune
/// let x = Some("value");
/// assert_eq!(x.expect("fruits are healthy"), "value");
/// ```
///
/// ```rune,should_panic
/// let x = None;
/// x.expect("fruits are healthy"); // panics with `fruits are healthy`
/// ```
///
/// # Recommended Message Style
///
/// We recommend that `expect` messages are used to describe the reason you
/// _expect_ the `Option` should be `Some`.
///
/// ```rune,should_panic
/// # let slice = [];
/// let item = slice.get(0).expect("slice should not be empty");
/// ```
///
/// **Hint**: If you're having trouble remembering how to phrase expect error
/// messages remember to focus on the word "should" as in "env variable should
/// be set by blah" or "the given binary should be available and executable by
/// the current user".
///
/// For more detail on expect message styles and the reasoning behind our
/// recommendation please refer to the section on ["Common Message
/// Styles"](../../std/error/index.html#common-message-styles) in the
/// [`std::error`](../../std/error/index.html) module docs.
#[rune::function(instance)]
fn expect(option: Option<Value>, message: Value) -> VmResult<Value> {
    match option {
        Some(some) => VmResult::Ok(some),
        None => {
            let mut f = Formatter::new();
            vm_try!(message.string_display(&mut f));
            VmResult::err(Panic::custom(f.into_string()))
        }
    }
}

/// Returns `true` if the option is a [`Some`] value.
///
/// # Examples
///
/// ```rune
/// let x = Some(2);
/// assert_eq!(x.is_some(), true);
///
/// let x = None;
/// assert_eq!(x.is_some(), false);
/// ```
#[rune::function(instance)]
fn is_some(this: &Option<Value>) -> bool {
    this.is_some()
}

/// Returns `true` if the option is a [`None`] value.
///
/// # Examples
///
/// ```rune
/// let x = Some(2);
/// assert_eq!(x.is_none(), false);
///
/// let x = None;
/// assert_eq!(x.is_none(), true);
/// ```
#[rune::function(instance)]
fn is_none(this: &Option<Value>) -> bool {
    this.is_none()
}

/// Construct an iterator over an optional value.
///
/// # Examples
///
/// ```rune
/// let value = Some(1);
/// let it = value.iter();
///
/// assert_eq!(Some(1), it.next());
/// assert_eq!(None, it.next());
///
/// let value = None;
/// let it = value.iter();
///
/// assert_eq!(None, it.next());
/// ```
#[rune::function(instance)]
fn iter(option: Option<Value>) -> Iterator {
    Iterator::from_double_ended("std::option::Iter", option.into_iter())
}

/// Construct an iterator over an optional value.
///
/// # Examples
///
/// ```rune
/// let value = Some(1);
///
/// let out = [];
///
/// for v in value {
///     out.push(v);
/// }
///
/// assert_eq!(out, [1]);
/// ```
#[rune::function(instance, protocol = INTO_ITER)]
fn into_iter(option: Option<Value>) -> Iterator {
    __rune_fn__iter(option)
}

/// Returns [`None`] if the option is [`None`], otherwise calls `f` with the
/// wrapped value and returns the result.
///
/// Some languages call this operation flatmap.
///
/// # Examples
///
/// ```rune
/// fn sq_then_to_string(x) {
///     x.checked_mul(x).map(|sq| sq.to_string())
/// }
///
/// assert_eq!(Some(2).and_then(sq_then_to_string), Some(4.to_string()));
/// assert_eq!(Some(1_000_000_000_000_000_000).and_then(sq_then_to_string), None); // overflowed!
/// assert_eq!(None.and_then(sq_then_to_string), None);
/// ```
///
/// Often used to chain fallible operations that may return [`None`].
///
/// ```rune
/// let arr_2d = [["A0", "A1"], ["B0", "B1"]];
///
/// let item_0_1 = arr_2d.get(0).and_then(|row| row.get(1));
/// assert_eq!(item_0_1, Some("A1"));
///
/// let item_2_0 = arr_2d.get(2).and_then(|row| row.get(0));
/// assert_eq!(item_2_0, None);
/// ```
#[rune::function(instance)]
fn and_then(option: Option<Value>, then: Function) -> VmResult<Option<Value>> {
    match option {
        // no need to clone v, passing the same reference forward
        Some(v) => VmResult::Ok(vm_try!(then.call::<_, _>((v,)))),
        None => VmResult::Ok(None),
    }
}

/// Maps an `Option<T>` to `Option<U>` by applying a function to a contained
/// value (if `Some`) or returns `None` (if `None`).
///
/// # Examples
///
/// Calculates the length of an `Option<[String]>` as an
/// `Option<[usize]>`, consuming the original:
///
/// [String]: ../../std/string/struct.String.html "String"
///
/// ```rune
/// let maybe_some_string = Some(String::from("Hello, World!"));
/// // `Option::map` takes self *by value*, consuming `maybe_some_string`
/// let maybe_some_len = maybe_some_string.map(|s| s.len());
/// assert_eq!(maybe_some_len, Some(13));
///
/// let x = None;
/// assert_eq!(x.map(|s| s.len()), None);
/// ```
#[rune::function(instance)]
fn map(option: Option<Value>, then: Function) -> VmResult<Option<Value>> {
    match option {
        // no need to clone v, passing the same reference forward
        Some(v) => VmResult::Ok(Some(vm_try!(then.call::<_, _>((v,))))),
        None => VmResult::Ok(None),
    }
}

/// Takes the value out of the option, leaving a [`None`] in its place.
///
/// # Examples
///
/// ```rune
/// let x = Some(2);
/// let y = x.take();
/// assert_eq!(x, None);
/// assert_eq!(y, Some(2));
///
/// let x = None;
/// let y = x.take();
/// assert_eq!(x, None);
/// assert_eq!(y, None);
/// ```
#[rune::function(instance)]
fn take(option: &mut Option<Value>) -> Option<Value> {
    option.take()
}

/// Transposes an `Option` of a [`Result`] into a [`Result`] of an `Option`.
///
/// [`None`] will be mapped to `[Ok]\([None])`. `[Some]\([Ok]\(\_))` and
/// `[Some]\([Err]\(\_))` will be mapped to `[Ok]\([Some]\(\_))` and
/// `[Err]\(\_)`.
///
/// # Examples
///
/// ```rune
/// let x = Ok(Some(5));
/// let y = Some(Ok(5));
/// assert_eq!(x, y.transpose());
/// ```
#[rune::function(instance)]
fn transpose(this: Option<Value>) -> VmResult<Value> {
    let value = match this {
        Some(value) => value,
        None => {
            let none = Value::from(vm_try!(Shared::new(Option::<Value>::None)));
            let result = Value::from(vm_try!(Shared::new(Result::<Value, Value>::Ok(none))));
            return VmResult::Ok(result);
        }
    };

    let result = vm_try!(vm_try!(value.into_result()).take());

    match result {
        Ok(ok) => {
            let some = Value::from(vm_try!(Shared::new(Option::<Value>::Some(ok.clone()))));
            let result = Value::from(vm_try!(Shared::new(Result::<Value, Value>::Ok(some))));
            VmResult::Ok(result)
        }
        Err(err) => {
            let result = Value::from(vm_try!(Shared::new(Result::<Value, Value>::Err(
                err.clone()
            ))));
            VmResult::Ok(result)
        }
    }
}

/// Returns the contained [`Some`] value, consuming the `self` value.
///
/// Because this function may panic, its use is generally discouraged. Instead,
/// prefer to use pattern matching and handle the [`None`] case explicitly, or
/// call [`unwrap_or`], [`unwrap_or_else`], or [`unwrap_or_default`].
///
/// [`unwrap_or`]: Option::unwrap_or
/// [`unwrap_or_else`]: Option::unwrap_or_else
/// [`unwrap_or_default`]: Option::unwrap_or_default
///
/// # Panics
///
/// Panics if the self value equals [`None`].
///
/// # Examples
///
/// ```rune
/// let x = Some("air");
/// assert_eq!(x.unwrap(), "air");
/// ```
///
/// ```rune,should_panic
/// let x = None;
/// assert_eq!(x.unwrap(), "air"); // fails
/// ```
#[rune::function(instance)]
fn unwrap(option: Option<Value>) -> VmResult<Value> {
    match option {
        Some(some) => VmResult::Ok(some),
        None => VmResult::err(Panic::custom("Called `Option::unwrap()` on a `None` value")),
    }
}

/// Returns the contained [`Some`] value or a provided `default`.
///
/// Arguments passed to `unwrap_or` are eagerly evaluated; if you are passing
/// the result of a function call, it is recommended to use [`unwrap_or_else`],
/// which is lazily evaluated.
///
/// [`unwrap_or_else`]: Option::unwrap_or_else
///
/// # Examples
///
/// ```rune
/// assert_eq!(Some("car").unwrap_or("bike"), "car");
/// assert_eq!(None.unwrap_or("bike"), "bike");
/// ```
#[rune::function(instance)]
fn unwrap_or(this: Option<Value>, default: Value) -> Value {
    this.unwrap_or(default)
}

/// Returns the contained [`Some`] value or computes it from a closure.
///
/// # Examples
///
/// ```rune
/// let k = 10;
/// assert_eq!(Some(4).unwrap_or_else(|| 2 * k), 4);
/// assert_eq!(None.unwrap_or_else(|| 2 * k), 20);
/// ```
#[rune::function(instance)]
fn unwrap_or_else(this: Option<Value>, default: Function) -> VmResult<Value> {
    match this {
        Some(value) => VmResult::Ok(value),
        None => default.call(()),
    }
}

/// Transforms the `Option<T>` into a [`Result<T, E>`], mapping [`Some(v)`] to
/// [`Ok(v)`] and [`None`] to [`Err(err)`].
///
/// Arguments passed to `ok_or` are eagerly evaluated; if you are passing the
/// result of a function call, it is recommended to use [`ok_or_else`], which is
/// lazily evaluated.
///
/// [`Ok(v)`]: Ok
/// [`Err(err)`]: Err
/// [`Some(v)`]: Some
/// [`ok_or_else`]: Option::ok_or_else
///
/// # Examples
///
/// ```rune
/// let x = Some("foo");
/// assert_eq!(x.ok_or(0), Ok("foo"));
///
/// let x = None;
/// assert_eq!(x.ok_or(0), Err(0));
/// ```
#[rune::function(instance)]
fn ok_or(this: Option<Value>, err: Value) -> Result<Value, Value> {
    this.ok_or(err)
}

/// Transforms the `Option<T>` into a [`Result<T, E>`], mapping [`Some(v)`] to
/// [`Ok(v)`] and [`None`] to [`Err(err())`].
///
/// [`Ok(v)`]: Ok
/// [`Err(err())`]: Err
/// [`Some(v)`]: Some
///
/// # Examples
///
/// ```rune
/// let x = Some("foo");
/// assert_eq!(x.ok_or_else(|| 0), Ok("foo"));
///
/// let x = None;
/// assert_eq!(x.ok_or_else(|| 0), Err(0));
/// ```
#[rune::function(instance)]
fn ok_or_else(this: Option<Value>, err: Function) -> VmResult<Result<Value, Value>> {
    match this {
        Some(value) => VmResult::Ok(Ok(value)),
        None => VmResult::Ok(Err(vm_try!(err.call(())))),
    }
}

/// Using [`Option`] with the try protocol.
///
/// # Examples
///
/// ```rune
/// fn maybe_add_one(value) {
///     Some(value? + 1)
/// }
///
/// assert_eq!(maybe_add_one(Some(4)), Some(5));
/// assert_eq!(maybe_add_one(None), None);
/// ```
#[rune::function(keep, instance, protocol = TRY)]
pub(crate) fn option_try(this: Option<Value>) -> VmResult<ControlFlow> {
    VmResult::Ok(match this {
        Some(value) => ControlFlow::Continue(value),
        None => ControlFlow::Break(Value::Option(vm_try!(Shared::new(None)))),
    })
}
