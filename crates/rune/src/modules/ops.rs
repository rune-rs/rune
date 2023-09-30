//! The `std::ops` module.

use core::cmp::Ordering;

use once_cell::sync::OnceCell;
use rune_alloc::hash_map::RandomState;

use crate as rune;
use crate::runtime::{
    ControlFlow, EnvProtocolCaller, Function, Generator, GeneratorState, Hasher, Iterator, Range,
    RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive, Value, Vm, VmResult,
};
use crate::{ContextError, Module};

static STATE: OnceCell<RandomState> = OnceCell::new();

#[rune::module(::std::ops)]
/// Overloadable operators.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;

    {
        m.ty::<RangeFrom>()?;
        m.function_meta(RangeFrom::iter__meta)?;
        m.function_meta(RangeFrom::contains__meta)?;
        m.function_meta(RangeFrom::into_iter__meta)?;
        m.function_meta(RangeFrom::partial_eq__meta)?;
        m.function_meta(RangeFrom::eq__meta)?;
        m.function_meta(RangeFrom::partial_cmp__meta)?;
        m.function_meta(RangeFrom::cmp__meta)?;
    }

    {
        m.ty::<RangeFull>()?;
        m.function_meta(RangeFull::contains)?;
    }

    {
        m.ty::<RangeInclusive>()?;
        m.function_meta(RangeInclusive::iter__meta)?;
        m.function_meta(RangeInclusive::contains__meta)?;
        m.function_meta(RangeInclusive::into_iter__meta)?;
        m.function_meta(RangeInclusive::partial_eq__meta)?;
        m.function_meta(RangeInclusive::eq__meta)?;
        m.function_meta(RangeInclusive::partial_cmp__meta)?;
        m.function_meta(RangeInclusive::cmp__meta)?;
    }

    {
        m.ty::<RangeToInclusive>()?;
        m.function_meta(RangeToInclusive::contains__meta)?;
        m.function_meta(RangeToInclusive::partial_eq__meta)?;
        m.function_meta(RangeToInclusive::eq__meta)?;
        m.function_meta(RangeToInclusive::partial_cmp__meta)?;
        m.function_meta(RangeToInclusive::cmp__meta)?;
    }

    {
        m.ty::<RangeTo>()?;
        m.function_meta(RangeTo::contains__meta)?;
        m.function_meta(RangeTo::partial_eq__meta)?;
        m.function_meta(RangeTo::eq__meta)?;
        m.function_meta(RangeTo::partial_cmp__meta)?;
        m.function_meta(RangeTo::cmp__meta)?;
    }

    {
        m.ty::<Range>()?;
        m.function_meta(Range::iter__meta)?;
        m.function_meta(Range::into_iter__meta)?;
        m.function_meta(Range::contains__meta)?;
        m.function_meta(Range::partial_eq__meta)?;
        m.function_meta(Range::eq__meta)?;
        m.function_meta(Range::partial_cmp__meta)?;
        m.function_meta(Range::cmp__meta)?;
    }

    {
        m.ty::<ControlFlow>()?;
    }

    m.ty::<Function>()?;

    {
        m.ty::<Generator<Vm>>()?;
        m.function_meta(generator_next)?;
        m.function_meta(generator_resume)?;
        m.function_meta(generator_iter)?;
        m.function_meta(generator_into_iter)?;
    }

    {
        m.generator_state(["GeneratorState"])?
            .docs(["Enum indicating the state of a generator."])?;

        m.function_meta(generator_state_partial_eq)?;
        m.function_meta(generator_state_eq)?;
    }

    m.function_meta(partial_eq)?;
    m.function_meta(eq)?;
    m.function_meta(partial_cmp)?;
    m.function_meta(cmp)?;
    m.function_meta(hash)?;
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

/// Advance a generator producing the next value yielded.
///
/// Unlike [`Generator::resume`], this can only consume the yielded values.
///
/// # Examples
///
/// ```rune
/// use std::ops::{Generator, GeneratorState};
///
/// fn generate() {
///     yield 1;
///     yield 2;
/// }
///
/// let g = generate();
///
/// assert_eq!(g.next(), Some(1));
/// assert_eq!(g.next(), Some(2));
/// assert_eq!(g.next(), None);
/// ``
#[rune::function(instance, path = next)]
fn generator_next(this: &mut Generator<Vm>) -> VmResult<Option<Value>> {
    this.next()
}

/// Advance a generator producing the next [`GeneratorState`].
///
/// # Examples
///
/// ```rune
/// use std::ops::{Generator, GeneratorState};
///
/// fn generate() {
///     let n = yield 1;
///     yield 2 + n;
/// }
///
/// let g = generate();
///
/// assert_eq!(g.resume(()), GeneratorState::Yielded(1));
/// assert_eq!(g.resume(1), GeneratorState::Yielded(3));
/// assert_eq!(g.resume(()), GeneratorState::Complete(()));
/// ``
#[rune::function(instance, path = resume)]
fn generator_resume(this: &mut Generator<Vm>, value: Value) -> VmResult<GeneratorState> {
    this.resume(value)
}

#[rune::function(instance, path = iter)]
fn generator_iter(this: Generator<Vm>) -> Iterator {
    this.rune_iter()
}

#[rune::function(instance, protocol = INTO_ITER)]
fn generator_into_iter(this: Generator<Vm>) -> Iterator {
    this.rune_iter()
}

/// Test for partial equality over a generator state.
///
/// # Examples
///
/// ```rune
/// use std::ops::{Generator, GeneratorState};
///
/// fn generate() {
///     let n = yield 1;
///     yield 2 + n;
/// }
///
/// let g = generate();
///
/// assert_eq!(g.resume(()), GeneratorState::Yielded(1));
/// assert_eq!(g.resume(1), GeneratorState::Yielded(3));
/// assert_eq!(g.resume(()), GeneratorState::Complete(()));
/// ``
#[rune::function(instance, protocol = PARTIAL_EQ)]
fn generator_state_partial_eq(this: &GeneratorState, other: &GeneratorState) -> VmResult<bool> {
    this.partial_eq_with(other, &mut EnvProtocolCaller)
}

/// Test for total equality over a generator state.
///
/// # Examples
///
/// ```rune
/// use std::ops::{Generator, GeneratorState};
/// use std::ops::eq;
///
/// fn generate() {
///     let n = yield 1;
///     yield 2 + n;
/// }
///
/// let g = generate();
///
/// assert!(eq(g.resume(()), GeneratorState::Yielded(1)));
/// assert!(eq(g.resume(1), GeneratorState::Yielded(3)));
/// assert!(eq(g.resume(()), GeneratorState::Complete(())));
/// ``
#[rune::function(instance, protocol = EQ)]
fn generator_state_eq(this: &GeneratorState, other: &GeneratorState) -> VmResult<bool> {
    this.eq_with(other, &mut EnvProtocolCaller)
}
