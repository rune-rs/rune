use core::fmt;
use core::iter;

use crate as rune;
use crate::alloc::clone::TryClone;
use crate::runtime::{GeneratorState, Value, Vm, VmError, VmErrorKind, VmExecution, VmResult};
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
#[rune(crate, impl_params = [Vm], item = ::std::ops::generator)]
pub struct Generator<T = Vm>
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
        let Some(execution) = self.execution.as_mut() else {
            return VmResult::Ok(None);
        };

        let state = if execution.is_resumed() {
            vm_try!(execution.resume_with(Value::empty()))
        } else {
            vm_try!(execution.resume())
        };

        VmResult::Ok(match state {
            GeneratorState::Yielded(value) => Some(value),
            GeneratorState::Complete(_) => {
                self.execution = None;
                None
            }
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

impl<T> TryClone for Generator<T>
where
    T: TryClone + AsRef<Vm> + AsMut<Vm>,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, rune_alloc::Error> {
        Ok(Self {
            execution: self.execution.try_clone()?,
        })
    }
}
