//! Overloadable operators and associated types.

use crate as rune;
use crate::runtime::generator::Iter;
use crate::runtime::{EnvProtocolCaller, Generator, GeneratorState, Value, Vm, VmResult};
use crate::{ContextError, Module};

/// Types related to generators.
#[rune::module(::std::ops::generator)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;

    {
        m.ty::<Generator<Vm>>()?.docs(docstring! {
            /// A generator produced by a generator function.
            ///
            /// Generator are functions or closures which contain the `yield`
            /// expressions.
            ///
            /// # Examples
            ///
            /// ```rune
            /// use std::ops::generator::Generator;
            ///
            /// let f = |n| {
            ///     yield n;
            ///     yield n + 1;
            /// };
            ///
            /// let g = f(10);
            ///
            /// assert!(g is Generator);
            /// ```
        })?;
        m.function_meta(generator_next)?;
        m.function_meta(generator_resume)?;
        m.function_meta(generator_iter)?;
        m.function_meta(generator_into_iter)?;

        m.ty::<Iter>()?.docs(docstring! {
            /// An iterator over a generator.
        })?;
        m.function_meta(Iter::next__meta)?;
        m.implement_trait::<Iter>(rune::item!(::std::iter::Iterator))?;
    }

    {
        m.generator_state(["GeneratorState"])?
            .docs(["Enum indicating the state of a generator."])?;

        m.function_meta(generator_state_partial_eq)?;
        m.implement_trait::<GeneratorState>(rune::item!(::std::cmp::PartialEq))?;

        m.function_meta(generator_state_eq)?;
        m.implement_trait::<GeneratorState>(rune::item!(::std::cmp::Eq))?;
    }

    Ok(m)
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

/// Convert a generator into an iterator.
///
/// # Examples
///
/// ```rune
/// fn count_numbers(limit) {
///     for n in 0..limit.unwrap_or(10) {
///         yield n;
///     }
/// }
///
/// assert_eq!(count_numbers(None).iter().collect::<Vec>(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
/// assert_eq!(count_numbers(Some(2)).iter().collect::<Vec>(), [0, 1]);
/// ```
#[rune::function(instance, path = iter)]
#[inline]
fn generator_iter(this: Generator<Vm>) -> Iter {
    this.rune_iter()
}

#[rune::function(instance, protocol = INTO_ITER)]
#[inline]
fn generator_into_iter(this: Generator<Vm>) -> Iter {
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
