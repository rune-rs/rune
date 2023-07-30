use crate::compile::Named;
use crate::module::InstallWith;
use crate::runtime::{RawStr, Value, VmResult};

/// The state of a generator.
///
/// ```
/// use rune::{Value, Vm};
/// use rune::runtime::{Generator, GeneratorState};
/// use std::sync::Arc;
///
/// let mut sources = rune::sources! {
///     entry => {
///         pub fn main() {
///             let n = yield 1;
///             let out = yield n + 1;
///             out
///         }
///     }
/// };
///
/// let unit = rune::prepare(&mut sources).build()?;
///
/// let mut vm = Vm::without_runtime(Arc::new(unit));
/// let mut execution = vm.execute(["main"], ())?;
///
/// // Initial resume doesn't take a value.
/// let first = match execution.resume().into_result()? {
///     GeneratorState::Yielded(first) => rune::from_value::<i64>(first)?,
///     GeneratorState::Complete(..) => panic!("generator completed"),
/// };
///
/// assert_eq!(first, 1);
///
/// // Additional resumes require a value.
/// let second = match execution.resume_with(Value::from(2i64)).into_result()? {
///     GeneratorState::Yielded(second) => rune::from_value::<i64>(second)?,
///     GeneratorState::Complete(..) => panic!("generator completed"),
/// };
///
/// assert_eq!(second, 3);
///
/// let ret = match execution.resume_with(Value::from(42i64)).into_result()? {
///     GeneratorState::Complete(ret) => rune::from_value::<i64>(ret)?,
///     GeneratorState::Yielded(..) => panic!("generator yielded"),
/// };
///
/// assert_eq!(ret, 42);
/// # Ok::<_, rune::Error>(())
/// ```
#[derive(Debug)]
pub enum GeneratorState {
    /// The generator yielded.
    Yielded(Value),
    /// The generator completed.
    Complete(Value),
}

impl GeneratorState {
    /// Test if the state is yielded.
    pub fn is_yielded(&self) -> bool {
        matches!(self, Self::Yielded(..))
    }

    /// Test if the state is complete.
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete(..))
    }
}

from_value!(GeneratorState, into_generator_state);

impl Named for GeneratorState {
    const BASE_NAME: RawStr = RawStr::from_str("GeneratorState");
}

impl InstallWith for GeneratorState {}
