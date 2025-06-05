use crate as rune;
use crate::alloc::clone::TryClone;
use crate::Any;

use super::{ProtocolCaller, Value, VmError};

/// The state of a generator.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use rune::{Value, Vm};
/// use rune::runtime::GeneratorState;
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
/// let mut generator = vm.execute(["main"], ())?.into_generator();
///
/// // Initial resume doesn't take a value.
/// let first = match generator.resume(Value::empty())? {
///     GeneratorState::Yielded(first) => rune::from_value::<i64>(first)?,
///     GeneratorState::Complete(..) => panic!("generator completed"),
/// };
///
/// assert_eq!(first, 1);
///
/// // Additional resumes require a value.
/// let second = match generator.resume(rune::to_value(2i64)?)? {
///     GeneratorState::Yielded(second) => rune::from_value::<i64>(second)?,
///     GeneratorState::Complete(..) => panic!("generator completed"),
/// };
///
/// assert_eq!(second, 3);
///
/// let ret = match generator.resume(rune::to_value(42i64)?)? {
///     GeneratorState::Complete(ret) => rune::from_value::<i64>(ret)?,
///     GeneratorState::Yielded(..) => panic!("generator yielded"),
/// };
///
/// assert_eq!(ret, 42);
/// # Ok::<_, rune::support::Error>(())
/// ```
///
/// An asynchronous generator, also known as a stream:
///
/// ```
/// use std::sync::Arc;
///
/// use rune::{Value, Vm};
/// use rune::runtime::GeneratorState;
///
/// let mut sources = rune::sources! {
///     entry => {
///         pub async fn main() {
///             let n = yield 1;
///             let out = yield n + 1;
///             out
///         }
///     }
/// };
///
/// # futures_executor::block_on(async move {
/// let unit = rune::prepare(&mut sources).build()?;
///
/// let mut vm = Vm::without_runtime(Arc::new(unit));
/// let mut stream = vm.execute(["main"], ())?.into_stream();
///
/// // Initial resume doesn't take a value.
/// let first = match stream.resume(Value::empty()).await? {
///     GeneratorState::Yielded(first) => rune::from_value::<i64>(first)?,
///     GeneratorState::Complete(..) => panic!("stream completed"),
/// };
///
/// assert_eq!(first, 1);
///
/// // Additional resumes require a value.
/// let second = match stream.resume(rune::to_value(2i64)?).await? {
///     GeneratorState::Yielded(second) => rune::from_value::<i64>(second)?,
///     GeneratorState::Complete(..) => panic!("stream completed"),
/// };
///
/// assert_eq!(second, 3);
///
/// let ret = match stream.resume(rune::to_value(42i64)?).await? {
///     GeneratorState::Complete(ret) => rune::from_value::<i64>(ret)?,
///     GeneratorState::Yielded(..) => panic!("stream yielded"),
/// };
///
/// assert_eq!(ret, 42);
/// # Ok::<_, rune::support::Error>(())
/// # })?;
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Any, Debug, TryClone)]
#[rune(item = ::std::ops::generator)]
pub enum GeneratorState {
    /// The generator yielded.
    #[rune(constructor)]
    Yielded(#[rune(get, set)] Value),
    /// The generator completed.
    #[rune(constructor)]
    Complete(#[rune(get, set)] Value),
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

    pub(crate) fn partial_eq_with(
        &self,
        other: &Self,
        caller: &mut dyn ProtocolCaller,
    ) -> Result<bool, VmError> {
        match (self, other) {
            (GeneratorState::Yielded(a), GeneratorState::Yielded(b)) => {
                Value::partial_eq_with(a, b, caller)
            }
            (GeneratorState::Complete(a), GeneratorState::Complete(b)) => {
                Value::partial_eq_with(a, b, caller)
            }
            _ => Ok(false),
        }
    }

    pub(crate) fn eq_with(
        &self,
        other: &Self,
        caller: &mut dyn ProtocolCaller,
    ) -> Result<bool, VmError> {
        match (self, other) {
            (GeneratorState::Yielded(a), GeneratorState::Yielded(b)) => {
                Value::eq_with(a, b, caller)
            }
            (GeneratorState::Complete(a), GeneratorState::Complete(b)) => {
                Value::eq_with(a, b, caller)
            }
            _ => Ok(false),
        }
    }
}
