use crate::{AccessError, Integer, Panic, ValueTypeInfo, VmError, VmErrorKind};
use thiserror::Error;
/// Errors raised by the execution of the virtual machine.
#[derive(Error, Debug)]
#[error(transparent)]
pub struct ValueError {
    kind: Box<ValueErrorKind>,
}

impl ValueError {
    /// Inspect the value error kind.
    pub fn kind(&self) -> &ValueErrorKind {
        &self.kind
    }

    /// Immediately convert value error into VmError for smuggled variants.
    pub fn unsmuggle_vm_error(self) -> Result<VmError, Self> {
        match *self.kind {
            ValueErrorKind::Panic { reason } => Ok(VmError::from(VmErrorKind::Panic { reason })),
            ValueErrorKind::VmError { error } => Ok(error),
            kind => Err(Self {
                kind: Box::new(kind),
            }),
        }
    }
}

impl<E> From<E> for ValueError
where
    ValueErrorKind: From<E>,
{
    fn from(err: E) -> Self {
        Self {
            kind: Box::new(ValueErrorKind::from(err)),
        }
    }
}

/// Value raised when interacting with a value.
#[derive(Debug, Error)]
pub enum ValueErrorKind {
    /// The virtual machine panicked for a specific reason.
    #[error("panicked `{reason}`")]
    Panic {
        /// The reason for the panic.
        reason: Panic,
    },
    /// A wrapped virtual machine error.
    #[error("{error}")]
    VmError {
        /// The source error.
        #[source]
        error: VmError,
    },
    /// Trying to access an inaccessible reference.
    #[error("failed to access value: {error}")]
    AccessError {
        /// Source error.
        #[from]
        error: AccessError,
    },
    /// Error raised when we expected a object.
    #[error("expected a object but found `{actual}`")]
    ExpectedObject {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a function pointer.
    #[error("expected a function pointer but found `{actual}`")]
    ExpectedFnPtr {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a value.
    #[error("expected a value of `{expected}` but found `{actual}`")]
    ExpectedAny {
        /// Expected type.
        expected: &'static str,
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a future.
    #[error("expected future, but found `{actual}`")]
    ExpectedFuture {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a generator.
    #[error("expected generator, but found `{actual}`")]
    ExpectedGenerator {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a generator state.
    #[error("expected generator state, but found `{actual}`")]
    ExpectedGeneratorState {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when expecting a unit.
    #[error("expected unit, but found `{actual}`")]
    ExpectedUnit {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when expecting an option.
    #[error("expected option, but found `{actual}`")]
    ExpectedOption {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expecting a result.
    #[error("expected result, but found `{actual}`")]
    ExpectedResult {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a boolean value.
    #[error("expected booleant, but found `{actual}`")]
    ExpectedBoolean {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a byte value.
    #[error("expected byte, but found `{actual}`")]
    ExpectedByte {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a char value.
    #[error("expected char, but found `{actual}`")]
    ExpectedChar {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when an integer value was expected.
    #[error("expected integer, but found `{actual}`")]
    ExpectedInteger {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a float value.
    #[error("expected float, but found `{actual}`")]
    ExpectedFloat {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a string.
    #[error("expected a string but found `{actual}`")]
    ExpectedString {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a byte string.
    #[error("expected a byte string but found `{actual}`")]
    ExpectedBytes {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a vector.
    #[error("expected a vector but found `{actual}`")]
    ExpectedVec {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a tuple.
    #[error("expected a tuple but found `{actual}`")]
    ExpectedTuple {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Failure to convert a number into an integer.
    #[error("failed to convert value `{from}` to integer `{to}`")]
    ValueToIntegerCoercionError {
        /// Number we tried to convert from.
        from: Integer,
        /// Number type we tried to convert to.
        to: &'static str,
    },
    /// Failure to convert an integer into a value.
    #[error("failed to convert integer `{from}` to value `{to}`")]
    IntegerToValueCoercionError {
        /// Number we tried to convert from.
        from: Integer,
        /// Number type we tried to convert to.
        to: &'static str,
    },
    /// Error raised when we expected an tuple of the given length.
    #[error("expected a tuple of length `{expected}`, but found one with length `{actual}`")]
    ExpectedTupleLength {
        /// The actual length observed.
        actual: usize,
        /// The expected tuple length.
        expected: usize,
    },
    /// Internal error that happens when we run out of items in a list.
    #[error("unexpectedly ran out of items to iterate over")]
    IterationError,
}
