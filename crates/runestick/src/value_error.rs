use crate::{AccessError, Integer, Panic, ReflectValueType, ValueTypeInfo, VmError, VmErrorKind};
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
    /// Error raised when we expected one type, but got another.
    #[error("expected `{expected}`, but found `{actual}`")]
    Expected {
        /// The expected value type info.
        expected: ValueTypeInfo,
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a value.
    #[error("expected `Any` type, but found `{actual}`")]
    ExpectedAny {
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

impl ValueError {
    /// Construct an expected error.
    pub fn expected<T>(actual: ValueTypeInfo) -> Self
    where
        T: ReflectValueType,
    {
        Self::from(ValueErrorKind::Expected {
            expected: T::value_type_info(),
            actual,
        })
    }

    /// Construct an expected any error.
    pub fn expected_any(actual: ValueTypeInfo) -> Self {
        Self::from(ValueErrorKind::ExpectedAny { actual })
    }
}
