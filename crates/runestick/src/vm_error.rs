use crate::panic::BoxedPanic;
use crate::{
    AccessError, Hash, Item, Panic, Protocol, StackError, TypeInfo, TypeOf, Unit, Value, VmHaltInfo,
};
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

/// Errors raised by the execution of the virtual machine.
#[derive(Error, Debug)]
#[error(transparent)]
pub struct VmError {
    kind: Box<VmErrorKind>,
}

impl VmError {
    /// Return an error encapsulating a panic.
    pub fn panic<D>(message: D) -> Self
    where
        D: BoxedPanic,
    {
        Self::from(VmErrorKind::Panic {
            reason: Panic::custom(message),
        })
    }

    /// Bad argument.
    pub fn bad_argument<T>(arg: usize, value: &Value) -> Result<Self, VmError>
    where
        T: TypeOf,
    {
        Ok(Self::from(VmErrorKind::BadArgumentType {
            arg,
            expected: T::type_info(),
            actual: value.type_info()?,
        }))
    }

    /// Construct an expected error.
    pub fn expected<T>(actual: TypeInfo) -> Self
    where
        T: TypeOf,
    {
        Self::from(VmErrorKind::Expected {
            expected: T::type_info(),
            actual,
        })
    }

    /// Construct an expected any error.
    pub fn expected_any(actual: TypeInfo) -> Self {
        Self::from(VmErrorKind::ExpectedAny { actual })
    }

    /// Access the underlying error kind.
    pub fn kind(&self) -> &VmErrorKind {
        &*self.kind
    }

    /// Convert into an unwinded vm error.
    pub fn into_unwinded(self, unit: &Arc<Unit>, ip: usize) -> Self {
        if let VmErrorKind::Unwound { .. } = &*self.kind {
            return self;
        }

        Self::from(VmErrorKind::Unwound {
            kind: self.kind,
            unit: unit.clone(),
            ip,
        })
    }

    /// Unpack an unwinded error, if it is present.
    pub fn into_unwound(self) -> (Self, Option<(Arc<Unit>, usize)>) {
        match *self.kind {
            VmErrorKind::Unwound { kind, unit, ip } => {
                let error = Self { kind };
                (error, Some((unit, ip)))
            }
            kind => (Self::from(kind), None),
        }
    }

    /// Unsmuggles the vm error, returning Ok(Self) in case the error is
    /// critical and should be propagated unaltered.
    pub fn unpack_critical(self) -> Result<Self, Self> {
        if self.is_critical() {
            Err(self)
        } else {
            Ok(self)
        }
    }

    /// Test if the error is critical and should be propagated unaltered or not.
    ///
    /// Returns `true` if the error should be propagated.
    fn is_critical(&self) -> bool {
        match &*self.kind {
            VmErrorKind::Panic { .. } => true,
            VmErrorKind::Unwound { .. } => true,
            _ => false,
        }
    }
}

impl<E> From<E> for VmError
where
    VmErrorKind: From<E>,
{
    fn from(err: E) -> Self {
        Self {
            kind: Box::new(VmErrorKind::from(err)),
        }
    }
}

/// The kind of error encountered.
#[derive(Debug, Error)]
pub enum VmErrorKind {
    /// A vm error that was propagated from somewhere else.
    ///
    /// In order to represent this, we need to preserve the instruction pointer
    /// and eventually unit from where the error happened.
    #[error("{kind} (at {ip})")]
    Unwound {
        /// The wrapper error.
        kind: Box<VmErrorKind>,
        /// Associated unit.
        unit: Arc<Unit>,
        /// The instruction pointer of where the original error happened.
        ip: usize,
    },
    /// The virtual machine panicked for a specific reason.
    #[error("panicked `{reason}`")]
    Panic {
        /// The reason for the panic.
        reason: Panic,
    },
    /// Raised when we try to access an empty execution.
    #[error("no running virtual machines")]
    NoRunningVm,
    /// The virtual machine stopped for an unexpected reason.
    #[error("halted for unexpected reason `{halt}`")]
    Halted {
        /// The reason why the virtual machine stopped.
        halt: VmHaltInfo,
    },
    /// Error raised when external format function results in error.
    #[error("failed to format argument")]
    FormatError,
    /// Error raised when interacting with the stack.
    #[error("stack error: {error}")]
    StackError {
        /// The source error.
        #[from]
        error: StackError,
    },
    /// The virtual machine encountered a numerical overflow.
    #[error("numerical overflow")]
    Overflow,
    /// The virtual machine encountered a numerical underflow.
    #[error("numerical underflow")]
    Underflow,
    /// The virtual machine encountered a divide-by-zero.
    #[error("division by zero")]
    DivideByZero,
    /// Failure to lookup function.
    #[error("missing entry `{item}` with hash `{hash}`")]
    MissingEntry {
        /// The name of the entrypoint.
        item: Item,
        /// Hash of function to look up.
        hash: Hash,
    },
    /// Failure to lookup function.
    #[error("missing function with hash `{hash}`")]
    MissingFunction {
        /// Hash of function to look up.
        hash: Hash,
    },
    /// Failure to lookup instance function.
    #[error("missing instance function `{hash}` for `{instance}`")]
    MissingInstanceFunction {
        /// Hash of function to look up.
        hash: Hash,
        /// The instance type we tried to look up function on.
        instance: TypeInfo,
    },
    /// Instruction pointer went out-of-bounds.
    #[error("instruction pointer is out-of-bounds")]
    IpOutOfBounds,
    /// Tried to await something on the stack which can't be await:ed.
    #[error("unsupported target for .await `{actual}`")]
    UnsupportedAwait {
        /// The actual target.
        actual: TypeInfo,
    },
    /// Unsupported binary operation.
    #[error("unsupported vm operation `{lhs} {op} {rhs}`")]
    UnsupportedBinaryOperation {
        /// Operation.
        op: &'static str,
        /// Left-hand side operator.
        lhs: TypeInfo,
        /// Right-hand side operator.
        rhs: TypeInfo,
    },
    /// Unsupported unary operation.
    #[error("unsupported vm operation `{op}{operand}`")]
    UnsupportedUnaryOperation {
        /// Operation.
        op: &'static str,
        /// Operand.
        operand: TypeInfo,
    },
    /// Protocol not implemented on type.
    #[error("`{actual}` does not implement the `{protocol}` protocol")]
    MissingProtocol {
        /// The missing protocol.
        protocol: Protocol,
        /// The encountered argument.
        actual: TypeInfo,
    },
    /// Indicates that a static string is missing for the given slot.
    #[error("static string slot `{slot}` does not exist")]
    MissingStaticString {
        /// Slot which is missing a static string.
        slot: usize,
    },
    /// Indicates that a static object keys is missing for the given slot.
    #[error("static object keys slot `{slot}` does not exist")]
    MissingStaticObjectKeys {
        /// Slot which is missing a static object keys.
        slot: usize,
    },
    /// Wrong number of arguments provided in call.
    #[error("wrong number of arguments `{actual}`, expected `{expected}`")]
    BadArgumentCount {
        /// The actual number of arguments.
        actual: usize,
        /// The expected number of arguments.
        expected: usize,
    },
    /// Failure to convert from one type to another.
    #[error("bad argument #{arg}, expected `{expected}` but got `{actual}`")]
    BadArgumentType {
        /// The argument location that was converted.
        arg: usize,
        /// The argument type we expected.
        expected: TypeInfo,
        /// The argument type we got.
        actual: TypeInfo,
    },
    /// Failure to convert from one type to another.
    #[error("bad argument #{arg} (expected `{to}`): {error}")]
    BadArgument {
        /// The underlying stack error.
        #[source]
        error: VmError,
        /// The argument location that was converted.
        arg: usize,
        /// The native type we attempt to convert to.
        to: &'static str,
    },
    /// Failure to convert return value.
    #[error("bad return value (expected `{ret}`): {error}")]
    BadReturn {
        /// Error describing the failed conversion.
        #[source]
        error: VmError,
        /// Type of the return value we attempted to convert.
        ret: &'static str,
    },
    /// An index set operation that is not supported.
    #[error("the index set operation `{target}[{index}] = {value}` is not supported")]
    UnsupportedIndexSet {
        /// The target type to set.
        target: TypeInfo,
        /// The index to set.
        index: TypeInfo,
        /// The value to set.
        value: TypeInfo,
    },
    /// An index get operation that is not supported.
    #[error("the index get operation `{target}[{index}]` is not supported")]
    UnsupportedIndexGet {
        /// The target type to get.
        target: TypeInfo,
        /// The index to get.
        index: TypeInfo,
    },
    /// An tuple index get operation that is not supported.
    #[error("the tuple index get operation is not supported on `{target}`")]
    UnsupportedTupleIndexGet {
        /// The target type we tried to perform the tuple indexing on.
        target: TypeInfo,
    },
    /// An tuple index set operation that is not supported.
    #[error("the tuple index set operation is not supported on `{target}`")]
    UnsupportedTupleIndexSet {
        /// The target type we tried to perform the tuple indexing on.
        target: TypeInfo,
    },
    /// An object slot index get operation that is not supported.
    #[error("field not available on `{target}`")]
    UnsupportedObjectSlotIndexGet {
        /// The target type we tried to perform the object indexing on.
        target: TypeInfo,
    },
    /// An is operation is not supported.
    #[error("`{value} is {test_type}` is not supported")]
    UnsupportedIs {
        /// The argument that is not supported.
        value: TypeInfo,
        /// The type that is not supported.
        test_type: TypeInfo,
    },
    /// Encountered a value that could not be called as a function
    #[error("`{actual_type}` cannot be called since it's not a function")]
    UnsupportedCallFn {
        /// The type that could not be called.
        actual_type: TypeInfo,
    },
    /// Tried to fetch an index in an object that doesn't exist.
    #[error("missing index by static string slot `{slot}` in object")]
    ObjectIndexMissing {
        /// The static string slot corresponding to the index that is missing.
        slot: usize,
    },
    /// Tried to access an index that was missing on a type.
    #[error("missing index `{}` on `{target}`")]
    MissingIndex {
        /// Type where field did not exist.
        target: TypeInfo,
        /// Index that we tried to access.
        index: VmIntegerRepr,
    },
    /// When we try to access a field that is missing.
    #[error("missing field `{field}` on `{target}`")]
    MissingField {
        /// Type where field did not exist.
        target: TypeInfo,
        /// Field that was missing.
        field: String,
    },
    /// When we try to access a field that is missing.
    #[error("missing dynamic field for struct field `{target}::{name}`")]
    MissingDynamicStructField {
        /// The target type where the field was missing.
        target: &'static str,
        /// The name of the missing field.
        name: &'static str,
    },
    /// When we try to access a tuple index that is missing.
    #[error("missing dynamic index #{index} in tuple struct `{target}`")]
    MissingDynamicStructTupleIndex {
        /// The target type where the field was missing.
        target: &'static str,
        /// The tuple index that is missing.
        index: usize,
    },
    /// Error raised when we try to unwrap something that is not an option or
    /// result.
    #[error("expected result or option with value to unwrap, but got `{actual}`")]
    UnsupportedUnwrap {
        /// The actual operand.
        actual: TypeInfo,
    },
    /// Error raised when we try to unwrap an Option that is not Some.
    #[error("expected Some value, but got `None`")]
    UnsupportedUnwrapNone,
    /// Error raised when we try to unwrap a Result that is not Ok.
    #[error("expected Ok value, but got `Err({err})`")]
    UnsupportedUnwrapErr {
        /// The error variant.
        err: TypeInfo,
    },
    /// Value is not supported for `is-value` test.
    #[error("expected result or option as value, but got `{actual}`")]
    UnsupportedIsValueOperand {
        /// The actual operand.
        actual: TypeInfo,
    },
    /// Trying to resume a generator that has completed.
    #[error("cannot resume a generator that has completed")]
    GeneratorComplete,
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
        expected: TypeInfo,
        /// The actual type found.
        actual: TypeInfo,
    },
    /// Error raised when we expected a value.
    #[error("expected `Any` type, but found `{actual}`")]
    ExpectedAny {
        /// The actual type observed instead.
        actual: TypeInfo,
    },
    /// Failure to convert a number into an integer.
    #[error("failed to convert value `{from}` to integer `{to}`")]
    ValueToIntegerCoercionError {
        /// Number we tried to convert from.
        from: VmIntegerRepr,
        /// Number type we tried to convert to.
        to: &'static str,
    },
    /// Failure to convert an integer into a value.
    #[error("failed to convert integer `{from}` to value `{to}`")]
    IntegerToValueCoercionError {
        /// Number we tried to convert from.
        from: VmIntegerRepr,
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

impl VmErrorKind {
    /// Unpack an unwound error, if it is present.
    pub fn as_unwound_ref(&self) -> (&Self, Option<(Arc<Unit>, usize)>) {
        match self {
            VmErrorKind::Unwound { kind, unit, ip } => (&*kind, Some((unit.clone(), *ip))),
            kind => (kind, None),
        }
    }
}

/// A type-erased rust number.
#[derive(Debug, Clone, Copy)]
pub enum VmIntegerRepr {
    /// `u8`
    U8(u8),
    /// `u16`
    U16(u16),
    /// `u32`
    U32(u32),
    /// `u64`
    U64(u64),
    /// `u128`
    U128(u128),
    /// `i8`
    I8(i8),
    /// `i16`
    I16(i16),
    /// `i32`
    I32(i32),
    /// `i64`
    I64(i64),
    /// `i128`
    I128(i128),
    /// `isize`
    Isize(isize),
    /// `usize`
    Usize(usize),
}

impl fmt::Display for VmIntegerRepr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::U8(n) => write!(fmt, "{}u8", n),
            Self::U16(n) => write!(fmt, "{}u16", n),
            Self::U32(n) => write!(fmt, "{}u32", n),
            Self::U64(n) => write!(fmt, "{}u64", n),
            Self::U128(n) => write!(fmt, "{}u128", n),
            Self::I8(n) => write!(fmt, "{}i8", n),
            Self::I16(n) => write!(fmt, "{}i16", n),
            Self::I32(n) => write!(fmt, "{}i32", n),
            Self::I64(n) => write!(fmt, "{}i64", n),
            Self::I128(n) => write!(fmt, "{}i128", n),
            Self::Isize(n) => write!(fmt, "{}isize", n),
            Self::Usize(n) => write!(fmt, "{}usize", n),
        }
    }
}
