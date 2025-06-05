use core::fmt;
use core::iter;

use crate as rune;
use crate::alloc::clone::TryClone;
use crate::runtime::{
    GeneratorState, Value, Vm, VmError, VmErrorKind, VmExecution, VmHaltInfo, VmOutcome,
};
use crate::Any;

/// A generator produced by a generator function.
///
/// Generator are functions or closures which contain the `yield` expressions.
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
#[derive(Any)]
#[rune(crate, item = ::std::ops::generator)]
pub struct Generator {
    execution: Option<VmExecution<Vm>>,
}

impl Generator {
    /// Construct a generator from a virtual machine.
    pub(crate) fn new(vm: Vm) -> Self {
        Self {
            execution: Some(VmExecution::new(vm)),
        }
    }

    /// Get the next value produced by this stream.
    pub fn next(&mut self) -> Result<Option<Value>, VmError> {
        let Some(execution) = self.execution.as_mut() else {
            return Ok(None);
        };

        let state = execution.resume().complete()?;

        match state {
            VmOutcome::Complete(_) => {
                self.execution = None;
                Ok(None)
            }
            VmOutcome::Yielded(value) => Ok(Some(value)),
            VmOutcome::Limited => Err(VmError::from(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            })),
        }
    }

    /// Resume the generator with a value and get the next [`GeneratorState`].
    pub fn resume(&mut self, value: Value) -> Result<GeneratorState, VmError> {
        let execution = self
            .execution
            .as_mut()
            .ok_or(VmErrorKind::GeneratorComplete)?;

        let outcome = execution.resume().with_value(value).complete()?;

        match outcome {
            VmOutcome::Complete(value) => {
                self.execution = None;
                Ok(GeneratorState::Complete(value))
            }
            VmOutcome::Yielded(value) => Ok(GeneratorState::Yielded(value)),
            VmOutcome::Limited => Err(VmError::from(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            })),
        }
    }
}

impl Generator {
    /// Convert into iterator
    pub fn rune_iter(self) -> Iter {
        self.into_iter()
    }
}

impl IntoIterator for Generator {
    type Item = Result<Value, VmError>;
    type IntoIter = Iter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter { generator: self }
    }
}

#[derive(Any)]
#[rune(item = ::std::ops::generator)]
pub struct Iter {
    generator: Generator,
}

impl Iter {
    #[rune::function(instance, keep, protocol = NEXT)]
    pub(crate) fn next(&mut self) -> Result<Option<Value>, VmError> {
        self.generator.next()
    }
}

impl iter::Iterator for Iter {
    type Item = Result<Value, VmError>;

    #[inline]
    fn next(&mut self) -> Option<Result<Value, VmError>> {
        match Iter::next(self) {
            Ok(Some(value)) => Some(Ok(value)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    }
}

impl fmt::Debug for Generator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Generator")
            .field("completed", &self.execution.is_none())
            .finish()
    }
}

impl TryClone for Generator {
    #[inline]
    fn try_clone(&self) -> Result<Self, rune_alloc::Error> {
        Ok(Self {
            execution: self.execution.try_clone()?,
        })
    }
}
