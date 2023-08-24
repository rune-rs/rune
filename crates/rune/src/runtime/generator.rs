use core::fmt;
use core::iter;

use crate as rune;
use crate::runtime::{GeneratorState, Iterator, Value, Vm, VmErrorKind, VmExecution, VmResult};
use crate::Any;

/// The return value of a function producing a generator.
///
/// Functions which contain the `yield` keyword produces generators.
///
/// # Examples
///
/// ```rune
/// use std::ops::Generator;
///
/// fn generate() {
///     yield 1;
///     yield 2;
/// }
///
/// let g = generate();
/// assert!(g is Generator)
/// ```
#[derive(Any)]
#[rune(builtin, static_type = GENERATOR_TYPE, from_value = Value::into_generator, from_value_params = [Vm])]
pub struct Generator<T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    execution: Option<VmExecution<T>>,
}

impl<T> Generator<T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    /// Construct a generator from a virtual machine.
    pub(crate) fn new(vm: T) -> Self {
        Self {
            execution: Some(VmExecution::new(vm)),
        }
    }

    /// Construct a generator from a complete execution.
    pub(crate) fn from_execution(execution: VmExecution<T>) -> Self {
        Self {
            execution: Some(execution),
        }
    }

    /// Get the next value produced by this stream.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> VmResult<Option<Value>> {
        VmResult::Ok(match vm_try!(self.resume(Value::EmptyTuple)) {
            GeneratorState::Yielded(value) => Some(value),
            GeneratorState::Complete(_) => None,
        })
    }

    /// Resume the generator with a value and get the next generator state.
    pub fn resume(&mut self, value: Value) -> VmResult<GeneratorState> {
        let execution = vm_try!(self
            .execution
            .as_mut()
            .ok_or(VmErrorKind::GeneratorComplete));

        let state = if execution.is_resumed() {
            vm_try!(execution.resume_with(value))
        } else {
            vm_try!(execution.resume())
        };

        if state.is_complete() {
            self.execution = None;
        }

        VmResult::Ok(state)
    }
}

impl Generator<&mut Vm> {
    /// Convert the current generator into one which owns its virtual machine.
    pub fn into_owned(self) -> Generator<Vm> {
        Generator {
            execution: self.execution.map(|e| e.into_owned()),
        }
    }
}

impl Generator<Vm> {
    /// Convert into iterator
    pub fn rune_iter(self) -> Iterator {
        Iterator::from("std::ops::generator::Iter", self.into_iter())
    }
}

impl IntoIterator for Generator<Vm> {
    type Item = VmResult<Value>;
    type IntoIter = Iter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter { generator: self }
    }
}

pub struct Iter {
    generator: Generator<Vm>,
}

impl iter::Iterator for Iter {
    type Item = VmResult<Value>;

    #[inline]
    fn next(&mut self) -> Option<VmResult<Value>> {
        match self.generator.next() {
            VmResult::Ok(Some(value)) => Some(VmResult::Ok(value)),
            VmResult::Ok(None) => None,
            VmResult::Err(error) => Some(VmResult::Err(error)),
        }
    }
}

impl<T> fmt::Debug for Generator<T>
where
    T: AsRef<Vm> + AsMut<Vm>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Generator")
            .field("completed", &self.execution.is_none())
            .finish()
    }
}
