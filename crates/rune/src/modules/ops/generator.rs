//! Overloadable operators and associated types.

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::runtime::generator::Iter;
use crate::runtime::{EnvProtocolCaller, Formatter, Generator, GeneratorState, Value, VmResult};
use crate::{docstring, vm_try, vm_write, ContextError, Module};

/// Types related to generators.
#[rune::module(::std::ops::generator)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module__meta)?;

    {
        m.ty::<Generator>()?;
        m.function_meta(generator_next__meta)?;
        m.function_meta(generator_resume__meta)?;
        m.function_meta(generator_iter__meta)?;
        m.function_meta(generator_into_iter__meta)?;
        m.function_meta(generator_debug__meta)?;
        m.function_meta(generator_clone__meta)?;
        m.implement_trait::<Generator>(rune::item!(::std::clone::Clone))?;

        m.ty::<Iter>()?.docs(docstring! {
            /// An iterator over a generator.
        })?;
        m.function_meta(Iter::next__meta)?;
        m.implement_trait::<Iter>(rune::item!(::std::iter::Iterator))?;
    }

    {
        m.ty::<GeneratorState>()?.docs(docstring! {
            /// Enum indicating the state of a generator.
        })?;

        m.function_meta(generator_state_partial_eq__meta)?;
        m.implement_trait::<GeneratorState>(rune::item!(::std::cmp::PartialEq))?;
        m.function_meta(generator_state_eq__meta)?;
        m.implement_trait::<GeneratorState>(rune::item!(::std::cmp::Eq))?;
        m.function_meta(generator_state_debug__meta)?;
        m.function_meta(generator_state_clone__meta)?;
        m.implement_trait::<GeneratorState>(rune::item!(::std::clone::Clone))?;
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
#[rune::function(keep, instance, path = next)]
fn generator_next(this: &mut Generator) -> VmResult<Option<Value>> {
    this.next()
}

/// Resumes the execution of this generator.
///
/// This function will resume execution of the generator or start execution if
/// it hasn't already. This call will return back into the generator's last
/// suspension point, resuming execution from the latest `yield`. The generator
/// will continue executing until it either yields or returns, at which point
/// this function will return.
///
/// # Return value
///
/// The `GeneratorState` enum returned from this function indicates what state
/// the generator is in upon returning. If the `Yielded` variant is returned
/// then the generator has reached a suspension point and a value has been
/// yielded out. Generators in this state are available for resumption at a
/// later point.
///
/// If `Complete` is returned then the generator has completely finished with
/// the value provided. It is invalid for the generator to be resumed again.
///
/// # Panics
///
/// This function may panic if it is called after the `Complete` variant has
/// been returned previously. While generator literals in the language are
/// guaranteed to panic on resuming after `Complete`, this is not guaranteed for
/// all implementations of the `Generator`.
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
#[rune::function(keep, instance, path = resume)]
fn generator_resume(this: &mut Generator, value: Value) -> VmResult<GeneratorState> {
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
#[rune::function(keep, instance, path = iter)]
#[inline]
fn generator_iter(this: Generator) -> Iter {
    this.rune_iter()
}

/// Construct an iterator over a generator.
///
/// # Examples
///
/// ```rune
/// fn count_numbers(limit) {
///     for n in 0..limit {
///         yield n;
///     }
/// }
///
/// let result = 0;
///
/// for n in count_numbers(3) {
///     result += n;
/// }
///
/// assert_eq!(result, 3);
/// ```
#[rune::function(keep, instance, protocol = INTO_ITER)]
#[inline]
fn generator_into_iter(this: Generator) -> Iter {
    this.rune_iter()
}

/// Debug print this generator.
///
/// # Examples
///
/// ```rune
/// use std::ops::GeneratorState;
///
/// fn generate() {
///     let n = yield 1;
///     yield 2 + n;
/// }
///
/// let a = generate();
///
/// println!("{a:?}");
/// ``
#[rune::function(keep, instance, protocol = DEBUG_FMT)]
fn generator_debug(this: &Generator, f: &mut Formatter) -> VmResult<()> {
    vm_write!(f, "{this:?}")
}

/// Clone a generator.
///
/// This clones the state of the generator too, allowing it to be resumed
/// independently.
///
/// # Examples
///
/// ```rune
/// use std::ops::GeneratorState;
///
/// fn generate() {
///     let n = yield 1;
///     yield 2 + n;
/// }
///
/// let a = generate();
///
/// assert_eq!(a.resume(()), GeneratorState::Yielded(1));
/// let b = a.clone();
/// assert_eq!(a.resume(2), GeneratorState::Yielded(4));
/// assert_eq!(b.resume(3), GeneratorState::Yielded(5));
///
/// assert_eq!(a.resume(()), GeneratorState::Complete(()));
/// assert_eq!(b.resume(()), GeneratorState::Complete(()));
/// ``
#[rune::function(keep, instance, protocol = CLONE)]
fn generator_clone(this: &Generator) -> VmResult<Generator> {
    VmResult::Ok(vm_try!(this.try_clone()))
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
#[rune::function(keep, instance, protocol = PARTIAL_EQ)]
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
#[rune::function(keep, instance, protocol = EQ)]
fn generator_state_eq(this: &GeneratorState, other: &GeneratorState) -> VmResult<bool> {
    this.eq_with(other, &mut EnvProtocolCaller)
}

/// Debug print this generator state.
///
/// # Examples
///
/// ```rune
/// use std::ops::GeneratorState;
///
/// let a = GeneratorState::Yielded(1);
/// let b = GeneratorState::Complete(());
///
/// println!("{a:?}");
/// println!("{b:?}");
/// ``
#[rune::function(keep, instance, protocol = DEBUG_FMT)]
fn generator_state_debug(this: &GeneratorState, f: &mut Formatter) -> VmResult<()> {
    match this {
        GeneratorState::Yielded(value) => {
            vm_try!(write!(f, "Yielded("));
            vm_try!(value.debug_fmt_with(f, &mut EnvProtocolCaller));
            vm_try!(write!(f, ")"));
        }
        GeneratorState::Complete(value) => {
            vm_try!(write!(f, "Complete("));
            vm_try!(value.debug_fmt_with(f, &mut EnvProtocolCaller));
            vm_try!(write!(f, ")"));
        }
    }

    VmResult::Ok(())
}

/// Clone a generator state.
///
/// # Examples
///
/// ```rune
/// use std::ops::GeneratorState;
///
/// let a = GeneratorState::Yielded(1);
/// let b = a.clone();
///
/// assert_eq!(a, b);
/// ``
#[rune::function(keep, instance, protocol = CLONE)]
fn generator_state_clone(this: &GeneratorState) -> VmResult<GeneratorState> {
    VmResult::Ok(vm_try!(this.try_clone()))
}
