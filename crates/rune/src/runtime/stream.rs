use core::fmt;

use crate as rune;
use crate::alloc;
use crate::alloc::clone::TryClone;
use crate::alloc::fmt::TryWrite;
use crate::runtime::{
    Formatter, GeneratorState, Mut, Value, Vm, VmError, VmErrorKind, VmExecution, VmHaltInfo,
    VmOutcome,
};
use crate::Any;

/// A stream produced by an async generator function.
///
/// Generator are async functions or closures which contain the `yield`
/// expressions.
///
/// # Examples
///
/// ```rune
/// use std::stream::Stream;
///
/// let f = async |n| {
///     yield n;
///     yield n + 1;
/// };
///
/// let g = f(10);
///
/// assert!(g is Stream);
/// ```
#[derive(Any)]
#[rune(item = ::std::stream)]
pub struct Stream {
    execution: Option<VmExecution<Vm>>,
}

impl Stream {
    /// Construct a stream from a virtual machine.
    pub(crate) fn new(vm: Vm) -> Self {
        Self {
            execution: Some(VmExecution::new(vm)),
        }
    }

    /// Get the next value produced by this stream.
    pub async fn next(&mut self) -> Result<Option<Value>, VmError> {
        let Some(execution) = self.execution.as_mut() else {
            return Ok(None);
        };

        match execution.resume().await? {
            VmOutcome::Complete(..) => {
                self.execution = None;
                Ok(None)
            }
            VmOutcome::Yielded(value) => Ok(Some(value)),
            VmOutcome::Limited => Err(VmError::new(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            })),
        }
    }

    /// Resume the stream with a value and return the next [`GeneratorState`].
    pub async fn resume(&mut self, value: Value) -> Result<GeneratorState, VmError> {
        let execution = self
            .execution
            .as_mut()
            .ok_or(VmError::new(VmErrorKind::GeneratorComplete))?;

        let outcome = execution.resume().with_value(value).await?;

        match outcome {
            VmOutcome::Complete(value) => {
                self.execution = None;
                Ok(GeneratorState::Complete(value))
            }
            VmOutcome::Yielded(value) => Ok(GeneratorState::Yielded(value)),
            VmOutcome::Limited => Err(VmError::new(VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            })),
        }
    }
}

impl Stream {
    /// Get the next value produced by this stream through an asynchronous
    /// iterator-like protocol.
    ///
    /// This function will resume execution until a value is produced through
    /// `GeneratorState::Yielded(value)`, at which point it will return
    /// `Some(value)`. Once `GeneratorState::Complete` is returned `None` will
    /// be returned.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::GeneratorState;
    ///
    /// async fn generate() {
    ///     yield 1;
    ///     yield 2;
    /// }
    ///
    /// let g = generate();
    ///
    /// assert_eq!(g.next().await, Some(1));
    /// assert_eq!(g.next().await, Some(2));
    /// assert_eq!(g.next().await, None);
    /// ``
    #[rune::function(keep, instance, path = Self::next)]
    pub(crate) async fn next_shared(mut this: Mut<Stream>) -> Result<Option<Value>, VmError> {
        this.next().await
    }

    /// Resumes the execution of this stream.
    ///
    /// This function will resume execution of the stream or start execution if
    /// it hasn't already. This call will return back into the stream's last
    /// suspension point, resuming execution from the latest `yield`. The stream
    /// will continue executing until it either yields or returns, at which
    /// point this function will return.
    ///
    /// # Return value
    ///
    /// The `GeneratorState` enum returned from this function indicates what
    /// state the stream is in upon returning. If the `Yielded` variant is
    /// returned then the stream has reached a suspension point and a value has
    /// been yielded out. Streams in this state are available for resumption at
    /// a later point.
    ///
    /// If `Complete` is returned then the stream has completely finished with
    /// the value provided. It is invalid for the stream to be resumed again.
    ///
    /// # Panics
    ///
    /// This function may panic if it is called after the `Complete` variant has
    /// been returned previously. While stream literals in the language are
    /// guaranteed to panic on resuming after `Complete`, this is not guaranteed
    /// for all implementations of the `Stream`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::GeneratorState;
    ///
    /// async fn generate() {
    ///     let n = yield 1;
    ///     yield 2 + n;
    /// }
    ///
    /// let g = generate();
    ///
    /// assert_eq!(g.resume(()).await, GeneratorState::Yielded(1));
    /// assert_eq!(g.resume(1).await, GeneratorState::Yielded(3));
    /// assert_eq!(g.resume(()).await, GeneratorState::Complete(()));
    /// ``
    #[rune::function(keep, instance, path = Self::resume)]
    pub(crate) async fn resume_shared(
        mut this: Mut<Stream>,
        value: Value,
    ) -> Result<GeneratorState, VmError> {
        this.resume(value).await
    }

    /// Debug print this stream
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
    fn debug(&self, f: &mut Formatter) -> alloc::Result<()> {
        write!(f, "{self:?}")
    }

    /// Clone a stream.
    ///
    /// This clones the state of the stream too, allowing it to be resumed
    /// independently.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::GeneratorState;
    ///
    /// async fn generate() {
    ///     let n = yield 1;
    ///     yield 2 + n;
    /// }
    ///
    /// let a = generate();
    ///
    /// assert_eq!(a.resume(()).await, GeneratorState::Yielded(1));
    /// let b = a.clone();
    /// assert_eq!(a.resume(2).await, GeneratorState::Yielded(4));
    /// assert_eq!(b.resume(3).await, GeneratorState::Yielded(5));
    ///
    /// assert_eq!(a.resume(()).await, GeneratorState::Complete(()));
    /// assert_eq!(b.resume(()).await, GeneratorState::Complete(()));
    /// ``
    #[rune::function(keep, instance, protocol = CLONE)]
    fn clone(&self) -> alloc::Result<Self> {
        self.try_clone()
    }
}

impl fmt::Debug for Stream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stream")
            .field("completed", &self.execution.is_none())
            .finish()
    }
}

impl TryClone for Stream {
    fn try_clone(&self) -> crate::alloc::Result<Self> {
        Ok(Self {
            execution: self.execution.try_clone()?,
        })
    }
}
