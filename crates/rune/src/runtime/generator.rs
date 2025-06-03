use core::fmt;
use core::iter;

use crate as rune;
use crate::alloc::clone::TryClone;
use crate::runtime::{GeneratorState, Value, Vm, VmError, VmErrorKind, VmExecution, VmResult};
use crate::{vm_try, Any};

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

    /// Construct a generator from a complete execution.
    pub(crate) fn from_execution(execution: VmExecution<Vm>) -> Self {
        Self {
            execution: Some(execution),
        }
    }

    /// Get the next value produced by this stream.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> VmResult<Option<Value>> {
        let Some(execution) = self.execution.as_mut() else {
            return VmResult::Ok(None);
        };

        let outcome = if execution.is_resumed() {
            vm_try!(execution.resume().with_value(Value::empty()).complete())
        } else {
            vm_try!(execution.resume().complete())
        };

        let state = vm_try!(outcome.into_generator_state());

        match state {
            GeneratorState::Yielded(value) => VmResult::Ok(Some(value)),
            GeneratorState::Complete(_) => {
                self.execution = None;
                VmResult::Ok(None)
            }
        }
    }

    /// Resume the generator with a value and get the next generator state.
    pub fn resume(&mut self, value: Value) -> VmResult<GeneratorState> {
        let execution = vm_try!(self
            .execution
            .as_mut()
            .ok_or(VmErrorKind::GeneratorComplete));

        let outcome = if execution.is_resumed() {
            vm_try!(execution.resume().with_value(value).complete())
        } else {
            vm_try!(execution.resume().complete())
        };

        let state = vm_try!(outcome.into_generator_state());

        if state.is_complete() {
            self.execution = None;
        }

        VmResult::Ok(state)
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
    pub(crate) fn next(&mut self) -> VmResult<Option<Value>> {
        self.generator.next()
    }
}

impl iter::Iterator for Iter {
    type Item = Result<Value, VmError>;

    #[inline]
    fn next(&mut self) -> Option<Result<Value, VmError>> {
        match Iter::next(self) {
            VmResult::Ok(Some(value)) => Some(Ok(value)),
            VmResult::Ok(None) => None,
            VmResult::Err(error) => Some(Err(error)),
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
