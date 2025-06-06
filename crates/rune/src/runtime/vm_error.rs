use core::convert::Infallible;
use core::fmt;

use rust_alloc::boxed::Box;

use crate::alloc::error::CustomError;
use crate::alloc::{self, String};
use crate::runtime::unit::{BadInstruction, BadJump};
use crate::sync::Arc;
use crate::{vm_error, Any, Hash, ItemBuf};

use super::{
    AccessError, AccessErrorKind, AnyObjError, AnyObjErrorKind, AnySequenceTakeError, BoxedPanic,
    CallFrame, DynArgsUsed, ExecutionState, Panic, Protocol, SliceError, StackError, StaticString,
    TypeInfo, TypeOf, Unit, Vm, VmHaltInfo,
};

vm_error!(VmError);

/// A virtual machine error which includes tracing information.
pub struct VmError {
    pub(crate) inner: Box<VmErrorInner>,
}

impl VmError {
    pub(crate) fn new<E>(error: E) -> Self
    where
        VmErrorKind: From<E>,
    {
        Self {
            inner: Box::new(VmErrorInner {
                error: VmErrorAt {
                    #[cfg(feature = "emit")]
                    index: 0,
                    kind: VmErrorKind::from(error),
                },
                chain: rust_alloc::vec::Vec::new(),
                stacktrace: rust_alloc::vec::Vec::new(),
            }),
        }
    }

    /// Construct an error containing a panic.
    pub fn panic<D>(message: D) -> Self
    where
        D: 'static + BoxedPanic,
    {
        Self::from(Panic::custom(message))
    }

    /// Construct an expectation error. The actual type received is `actual`,
    /// but we expected `E`.
    pub fn expected<E>(actual: TypeInfo) -> Self
    where
        E: ?Sized + TypeOf,
    {
        Self::from(VmErrorKind::expected::<E>(actual))
    }

    /// Get the location where the error happened.
    pub fn at(&self) -> &VmErrorAt {
        &self.inner.error
    }

    /// Get the full backtrace of errors and their corresponding instructions.
    pub fn chain(&self) -> &[VmErrorAt] {
        &self.inner.chain
    }

    /// Construct an overflow error.
    pub fn overflow() -> Self {
        Self::from(VmErrorKind::Overflow)
    }

    /// Get the first error location.
    pub fn first_location(&self) -> Option<&VmErrorLocation> {
        self.inner.stacktrace.first()
    }

    pub(crate) fn into_kind(self) -> VmErrorKind {
        self.inner.error.kind
    }

    /// Apply the given frame to the current result.
    pub(crate) fn with_vm<T>(result: Result<T, Self>, vm: &Vm) -> Result<T, Self> {
        match result {
            Ok(ok) => Ok(ok),
            Err(mut err) => {
                err.inner.stacktrace.push(VmErrorLocation {
                    unit: vm.unit().clone(),
                    ip: vm.last_ip(),
                    frames: vm.call_frames().to_vec(),
                });

                Err(err)
            }
        }
    }

    /// Add auxilliary errors if appropriate.
    #[inline]
    pub(crate) fn with_error<E>(mut self, error: E) -> Self
    where
        VmErrorKind: From<E>,
    {
        #[cfg(feature = "emit")]
        let index = self.inner.stacktrace.len();

        self.inner.chain.push(VmErrorAt {
            #[cfg(feature = "emit")]
            index,
            kind: VmErrorKind::from(error),
        });

        self
    }
}

impl fmt::Display for VmError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.error.fmt(f)
    }
}

impl fmt::Debug for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VmError")
            .field("error", &self.inner.error)
            .field("chain", &self.inner.chain)
            .field("stacktrace", &self.inner.stacktrace)
            .finish()
    }
}

impl core::error::Error for VmError {}

/// A single unit producing errors.
#[derive(Debug)]
#[non_exhaustive]
pub struct VmErrorLocation {
    /// Associated unit.
    pub unit: Arc<Unit>,
    /// Frozen instruction pointer.
    pub ip: usize,
    /// All lower call frames before the unwind trigger point
    pub frames: rust_alloc::vec::Vec<CallFrame>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct VmErrorAt {
    /// Index into the backtrace which contains information of what caused this error.
    #[cfg(feature = "emit")]
    index: usize,
    /// The kind of error.
    kind: VmErrorKind,
}

impl VmErrorAt {
    /// Get the instruction which caused the error.
    #[cfg(feature = "emit")]
    pub(crate) fn index(&self) -> usize {
        self.index
    }

    #[cfg(feature = "emit")]
    pub(crate) fn kind(&self) -> &VmErrorKind {
        &self.kind
    }
}

impl fmt::Display for VmErrorAt {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

#[non_exhaustive]
pub(crate) struct VmErrorInner {
    pub(crate) error: VmErrorAt,
    pub(crate) chain: rust_alloc::vec::Vec<VmErrorAt>,
    pub(crate) stacktrace: rust_alloc::vec::Vec<VmErrorLocation>,
}

/// A result produced by the virtual machine.
#[deprecated = "Use `Result<T, VmError>` directly instead."]
pub type VmResult<T> = Result<T, VmError>;

impl<E> From<E> for VmError
where
    VmErrorKind: From<E>,
{
    fn from(error: E) -> Self {
        Self::new(error)
    }
}

impl<E> From<CustomError<E>> for VmError
where
    VmError: From<E>,
{
    #[inline]
    fn from(error: CustomError<E>) -> Self {
        match error {
            CustomError::Custom(error) => Self::from(error),
            CustomError::Error(error) => VmError::new(error),
        }
    }
}

impl<const N: usize> From<[VmErrorKind; N]> for VmError {
    fn from(kinds: [VmErrorKind; N]) -> Self {
        let mut it = kinds.into_iter();

        let Some(first) = it.next() else {
            return VmError::panic("Cannot construct an empty collection of errors");
        };

        let mut chain = rust_alloc::vec::Vec::with_capacity(it.len());

        for kind in it {
            chain.push(VmErrorAt {
                #[cfg(feature = "emit")]
                index: 0,
                kind,
            });
        }

        Self {
            inner: Box::new(VmErrorInner {
                error: VmErrorAt {
                    #[cfg(feature = "emit")]
                    index: 0,
                    kind: first,
                },
                chain,
                stacktrace: rust_alloc::vec::Vec::new(),
            }),
        }
    }
}

impl From<Panic> for VmErrorKind {
    #[inline]
    fn from(reason: Panic) -> Self {
        VmErrorKind::Panic { reason }
    }
}

/// A expected error.
pub struct ExpectedType {
    pub(crate) expected: TypeInfo,
    pub(crate) actual: TypeInfo,
}

impl ExpectedType {
    /// Construct an expected error.
    pub(crate) fn new<T>(actual: TypeInfo) -> Self
    where
        T: ?Sized + TypeOf,
    {
        Self {
            expected: T::type_info(),
            actual,
        }
    }
}

vm_error!(RuntimeError);

/// An opaque simple runtime error.
#[cfg_attr(test, derive(PartialEq))]
pub struct RuntimeError {
    error: VmErrorKind,
}

impl RuntimeError {
    pub(crate) fn new(error: VmErrorKind) -> Self {
        Self { error }
    }

    pub(crate) fn into_vm_error_kind(self) -> VmErrorKind {
        self.error
    }

    /// Construct an error containing a panic.
    pub fn panic<D>(message: D) -> Self
    where
        D: 'static + BoxedPanic,
    {
        Self::new(VmErrorKind::from(Panic::custom(message)))
    }

    /// Bad argument count.
    pub fn bad_argument_count(actual: usize, expected: usize) -> Self {
        Self::new(VmErrorKind::BadArgumentCount { actual, expected })
    }

    /// Construct an expected error.
    pub fn expected<T>(actual: TypeInfo) -> Self
    where
        T: ?Sized + TypeOf,
    {
        Self::new(VmErrorKind::ExpectedType {
            expected: T::type_info(),
            actual,
        })
    }

    /// Construct an expected error from any.
    pub(crate) fn expected_any<T>(actual: TypeInfo) -> Self
    where
        T: Any,
    {
        Self::new(VmErrorKind::ExpectedType {
            expected: TypeInfo::any::<T>(),
            actual,
        })
    }

    /// Construct an expected any error.
    pub(crate) fn expected_any_obj(actual: TypeInfo) -> Self {
        Self::new(VmErrorKind::ExpectedAny { actual })
    }

    /// Indicate that a constant constructor is missing.
    pub(crate) fn missing_constant_constructor(hash: Hash) -> Self {
        Self::new(VmErrorKind::MissingConstantConstructor { hash })
    }

    pub(crate) fn expected_empty(actual: TypeInfo) -> Self {
        Self::new(VmErrorKind::ExpectedEmpty { actual })
    }

    pub(crate) fn expected_tuple(actual: TypeInfo) -> Self {
        Self::new(VmErrorKind::ExpectedTuple { actual })
    }

    pub(crate) fn expected_struct(actual: TypeInfo) -> Self {
        Self::new(VmErrorKind::ExpectedStruct { actual })
    }
}

impl fmt::Debug for RuntimeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}

impl From<Infallible> for RuntimeError {
    #[inline]
    fn from(error: Infallible) -> Self {
        match error {}
    }
}

impl From<AnySequenceTakeError> for RuntimeError {
    #[inline]
    fn from(value: AnySequenceTakeError) -> Self {
        match value {
            AnySequenceTakeError::Access(error) => Self::from(error),
            AnySequenceTakeError::Alloc(error) => Self::from(error),
        }
    }
}

impl From<VmError> for RuntimeError {
    #[inline]
    fn from(error: VmError) -> Self {
        Self::new(error.into_kind())
    }
}

impl From<alloc::Error> for RuntimeError {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        RuntimeError::from(VmErrorKind::from(error))
    }
}

impl From<alloc::alloc::AllocError> for RuntimeError {
    #[inline]
    fn from(error: alloc::alloc::AllocError) -> Self {
        RuntimeError::from(VmErrorKind::from(error))
    }
}

impl From<AnyObjError> for RuntimeError {
    #[inline]
    fn from(value: AnyObjError) -> Self {
        match value.into_kind() {
            AnyObjErrorKind::Alloc(error) => Self::from(error),
            AnyObjErrorKind::Cast(expected, actual) => Self::new(VmErrorKind::ExpectedType {
                expected: TypeInfo::any_type_info(expected),
                actual,
            }),
            AnyObjErrorKind::AccessError(error) => Self::from(error),
            AnyObjErrorKind::NotOwned(type_info) => Self::new(VmErrorKind::NotOwned { type_info }),
        }
    }
}

impl From<AccessError> for RuntimeError {
    #[inline]
    fn from(error: AccessError) -> Self {
        Self {
            error: VmErrorKind::from(error),
        }
    }
}

impl From<StackError> for RuntimeError {
    #[inline]
    fn from(error: StackError) -> Self {
        Self {
            error: VmErrorKind::from(error),
        }
    }
}

impl From<AccessErrorKind> for RuntimeError {
    #[inline]
    fn from(error: AccessErrorKind) -> Self {
        Self {
            error: VmErrorKind::from(AccessError::from(error)),
        }
    }
}

impl From<VmErrorKind> for RuntimeError {
    #[inline]
    fn from(error: VmErrorKind) -> Self {
        Self { error }
    }
}

impl From<ExpectedType> for RuntimeError {
    #[inline]
    fn from(expected: ExpectedType) -> Self {
        Self::from(VmErrorKind::ExpectedType {
            expected: expected.expected,
            actual: expected.actual,
        })
    }
}

impl fmt::Display for RuntimeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}

impl core::error::Error for RuntimeError {}

/// The kind of error encountered.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[doc(hidden)]
pub(crate) enum VmErrorKind {
    AllocError {
        error: alloc::Error,
    },
    AccessError {
        error: AccessError,
    },
    StackError {
        error: StackError,
    },
    SliceError {
        error: SliceError,
    },
    BadInstruction {
        error: BadInstruction,
    },
    BadJump {
        error: BadJump,
    },
    DynArgsUsed {
        error: DynArgsUsed,
    },
    Panic {
        reason: Panic,
    },
    NoRunningVm,
    Halted {
        halt: VmHaltInfo,
    },
    Overflow,
    Underflow,
    DivideByZero,
    MissingEntry {
        item: ItemBuf,
        hash: Hash,
    },
    MissingEntryHash {
        hash: Hash,
    },
    MissingFunction {
        hash: Hash,
    },
    MissingContextFunction {
        hash: Hash,
    },
    NotOwned {
        type_info: TypeInfo,
    },
    MissingProtocolFunction {
        protocol: &'static Protocol,
        instance: TypeInfo,
    },
    MissingInstanceFunction {
        hash: Hash,
        instance: TypeInfo,
    },
    IpOutOfBounds {
        ip: usize,
        length: usize,
    },
    UnsupportedBinaryOperation {
        op: &'static str,
        lhs: TypeInfo,
        rhs: TypeInfo,
    },
    UnsupportedUnaryOperation {
        op: &'static str,
        operand: TypeInfo,
    },
    MissingStaticString {
        slot: usize,
    },
    MissingStaticBytes {
        slot: usize,
    },
    MissingStaticObjectKeys {
        slot: usize,
    },
    MissingDropSet {
        set: usize,
    },
    MissingRtti {
        hash: Hash,
    },
    BadArgumentCount {
        actual: usize,
        expected: usize,
    },
    BadEnvironmentCount {
        actual: usize,
        expected: usize,
    },
    BadArgument {
        arg: usize,
    },
    UnsupportedIndexSet {
        target: TypeInfo,
        index: TypeInfo,
        value: TypeInfo,
    },
    UnsupportedIndexGet {
        target: TypeInfo,
        index: TypeInfo,
    },
    UnsupportedTupleIndexGet {
        target: TypeInfo,
        index: usize,
    },
    UnsupportedTupleIndexSet {
        target: TypeInfo,
    },
    UnsupportedObjectSlotIndexGet {
        target: TypeInfo,
        field: Arc<StaticString>,
    },
    UnsupportedObjectSlotIndexSet {
        target: TypeInfo,
        field: Arc<StaticString>,
    },
    UnsupportedIs {
        value: TypeInfo,
        test_type: TypeInfo,
    },
    UnsupportedAs {
        value: TypeInfo,
        type_hash: Hash,
    },
    UnsupportedCallFn {
        actual: TypeInfo,
    },
    ObjectIndexMissing {
        slot: usize,
    },
    MissingIndex {
        target: TypeInfo,
    },
    MissingIndexInteger {
        target: TypeInfo,
        index: VmIntegerRepr,
    },
    MissingIndexKey {
        target: TypeInfo,
    },
    OutOfRange {
        index: VmIntegerRepr,
        length: VmIntegerRepr,
    },
    UnsupportedTryOperand {
        actual: TypeInfo,
    },
    UnsupportedIterRangeInclusive {
        start: TypeInfo,
        end: TypeInfo,
    },
    UnsupportedIterRangeFrom {
        start: TypeInfo,
    },
    UnsupportedIterRange {
        start: TypeInfo,
        end: TypeInfo,
    },
    UnsupportedIterNextOperand {
        actual: TypeInfo,
    },
    ExpectedType {
        expected: TypeInfo,
        actual: TypeInfo,
    },
    ExpectedAny {
        actual: TypeInfo,
    },
    ExpectedNumber {
        actual: TypeInfo,
    },
    ExpectedEmpty {
        actual: TypeInfo,
    },
    ExpectedTuple {
        actual: TypeInfo,
    },
    ExpectedStruct {
        actual: TypeInfo,
    },
    MissingConstantConstructor {
        hash: Hash,
    },
    ValueToIntegerCoercionError {
        from: VmIntegerRepr,
        to: &'static str,
    },
    IntegerToValueCoercionError {
        from: VmIntegerRepr,
        to: &'static str,
    },
    ExpectedTupleLength {
        actual: usize,
        expected: usize,
    },
    ExpectedVecLength {
        actual: usize,
        expected: usize,
    },
    ConstNotSupported {
        actual: TypeInfo,
    },
    MissingInterfaceEnvironment,
    ExpectedExitedExecutionState {
        actual: ExecutionState,
    },
    GeneratorComplete,
    FutureCompleted,
    // Used in rune-macros.
    MissingVariant {
        name: String,
    },
    MissingField {
        target: TypeInfo,
        field: String,
    },
    MissingVariantName,
    MissingStructField {
        target: &'static str,
        name: &'static str,
    },
    MissingTupleIndex {
        target: &'static str,
        index: usize,
    },
    ExpectedVariant {
        actual: TypeInfo,
    },
    UnsupportedObjectFieldGet {
        target: TypeInfo,
    },
    IllegalFloatComparison {
        lhs: f64,
        rhs: f64,
    },
    IllegalFloatOperation {
        value: f64,
    },
    MissingCallFrame,
    IllegalFormat,
}

impl fmt::Display for VmErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VmErrorKind::AllocError { error } => error.fmt(f),
            VmErrorKind::AccessError { error } => error.fmt(f),
            VmErrorKind::StackError { error } => error.fmt(f),
            VmErrorKind::SliceError { error } => error.fmt(f),
            VmErrorKind::BadInstruction { error } => error.fmt(f),
            VmErrorKind::BadJump { error } => error.fmt(f),
            VmErrorKind::DynArgsUsed { error } => error.fmt(f),
            VmErrorKind::Panic { reason } => write!(f, "Panicked: {reason}"),
            VmErrorKind::NoRunningVm => write!(f, "No running virtual machines"),
            VmErrorKind::Halted { halt } => write!(f, "Halted for unexpected reason `{halt}`"),
            VmErrorKind::Overflow => write!(f, "Numerical overflow"),
            VmErrorKind::Underflow => write!(f, "Numerical underflow"),
            VmErrorKind::DivideByZero => write!(f, "Division by zero"),
            VmErrorKind::MissingEntry { item, hash } => {
                write!(f, "Missing entry `{item}` with hash `{hash}`")
            }
            VmErrorKind::MissingEntryHash { hash } => {
                write!(f, "Missing entry with hash `{hash}`")
            }
            VmErrorKind::MissingFunction { hash } => {
                write!(f, "Missing function with hash `{hash}`")
            }
            VmErrorKind::MissingContextFunction { hash } => {
                write!(f, "Missing context function with hash `{hash}`")
            }
            VmErrorKind::NotOwned { type_info } => {
                write!(f, "Cannot use owned operations for {type_info}")
            }
            VmErrorKind::MissingProtocolFunction { protocol, instance } => {
                write!(f, "Missing protocol function `{protocol}` for `{instance}`")
            }
            VmErrorKind::MissingInstanceFunction { hash, instance } => {
                write!(f, "Missing instance function `{hash}` for `{instance}`")
            }
            VmErrorKind::IpOutOfBounds { ip, length } => write!(
                f,
                "Instruction pointer `{ip}` is out-of-bounds `0-{length}`",
            ),
            VmErrorKind::UnsupportedBinaryOperation { op, lhs, rhs } => {
                write!(
                    f,
                    "Unsupported binary operation `{op}` on `{lhs}` and `{rhs}`",
                )
            }
            VmErrorKind::UnsupportedUnaryOperation { op, operand } => {
                write!(f, "Unsupported unary operation `{op}` on {operand}")
            }
            VmErrorKind::MissingStaticString { slot } => {
                write!(f, "Static string slot {slot} does not exist")
            }
            VmErrorKind::MissingStaticBytes { slot } => {
                write!(f, "Static bytes slot {slot} does not exist")
            }
            VmErrorKind::MissingStaticObjectKeys { slot } => {
                write!(f, "Static object keys slot {slot} does not exist")
            }
            VmErrorKind::MissingDropSet { set } => {
                write!(f, "Static drop set {set} does not exist")
            }
            VmErrorKind::MissingRtti { hash } => {
                write!(f, "Missing runtime information for type with hash `{hash}`")
            }
            VmErrorKind::BadArgumentCount { actual, expected } => {
                write!(f, "Wrong number of arguments {actual}, expected {expected}",)
            }
            VmErrorKind::BadEnvironmentCount { actual, expected } => write!(
                f,
                "Wrong environment size `{actual}`, expected `{expected}`",
            ),
            VmErrorKind::BadArgument { arg } => write!(f, "Bad argument #{arg}"),
            VmErrorKind::UnsupportedIndexSet {
                target,
                index,
                value,
            } => write!(
                f,
                "The index set operation `{target}[{index}] = {value}` is not supported",
            ),
            VmErrorKind::UnsupportedIndexGet { target, index } => write!(
                f,
                "The index get operation `{target}[{index}]` is not supported",
            ),
            VmErrorKind::UnsupportedTupleIndexGet { target, index } => write!(
                f,
                "The tuple index get {index} operation is not supported on `{target}`",
            ),
            VmErrorKind::UnsupportedTupleIndexSet { target } => write!(
                f,
                "The tuple index set operation is not supported on `{target}`",
            ),
            VmErrorKind::UnsupportedObjectSlotIndexGet { target, field } => {
                write!(f, "Field `{field}` not available on `{target}`")
            }
            VmErrorKind::UnsupportedObjectSlotIndexSet { target, field } => {
                write!(f, "Field `{field}` not available to set on `{target}`")
            }
            VmErrorKind::UnsupportedIs { value, test_type } => {
                write!(f, "Operation `{value} is {test_type}` is not supported")
            }
            VmErrorKind::UnsupportedAs { value, type_hash } => {
                write!(f, "Operation `{value} as {type_hash}` is not supported")
            }
            VmErrorKind::UnsupportedCallFn { actual } => write!(
                f,
                "Type `{actual}` cannot be called since it's not a function",
            ),
            VmErrorKind::ObjectIndexMissing { slot } => {
                write!(f, "Missing index by static string slot `{slot}`")
            }
            VmErrorKind::MissingIndex { target } => {
                write!(f, "Type `{target}` missing index")
            }
            VmErrorKind::MissingIndexInteger { target, index } => {
                write!(f, "Type `{target}` missing integer index `{index}`")
            }
            VmErrorKind::MissingIndexKey { target } => {
                write!(f, "Type `{target}` missing index")
            }
            VmErrorKind::OutOfRange { index, length } => write!(
                f,
                "Index out of bounds, the length is `{length}` but the index is `{index}`",
            ),
            VmErrorKind::UnsupportedTryOperand { actual } => {
                write!(f, "Type `{actual}` is not supported as try operand")
            }
            VmErrorKind::UnsupportedIterRangeInclusive { start, end } => {
                write!(f, "Cannot build an iterator out of {start}..={end}")
            }
            VmErrorKind::UnsupportedIterRangeFrom { start } => {
                write!(f, "Cannot build an iterator out of {start}..")
            }
            VmErrorKind::UnsupportedIterRange { start, end } => {
                write!(f, "Cannot build an iterator out of {start}..{end}")
            }
            VmErrorKind::UnsupportedIterNextOperand { actual } => {
                write!(f, "Type `{actual}` is not supported as iter-next operand")
            }
            VmErrorKind::ExpectedType { expected, actual } => {
                write!(f, "Expected type `{expected}` but found `{actual}`")
            }
            VmErrorKind::ExpectedAny { actual } => {
                write!(f, "Expected `Any` type, but found `{actual}`")
            }
            VmErrorKind::ExpectedNumber { actual } => {
                write!(f, "Expected number type, but found `{actual}`")
            }
            VmErrorKind::ExpectedEmpty { actual } => {
                write!(f, "Expected empty, but found `{actual}`")
            }
            VmErrorKind::ExpectedTuple { actual } => {
                write!(f, "Expected tuple, but found `{actual}`")
            }
            VmErrorKind::ExpectedStruct { actual } => {
                write!(f, "Expected struct, but found `{actual}`")
            }
            VmErrorKind::MissingConstantConstructor { hash } => {
                write!(f, "Missing constant constructor for type with hash {hash}")
            }
            VmErrorKind::ValueToIntegerCoercionError { from, to } => {
                write!(f, "Failed to convert value `{from}` to integer `{to}`")
            }
            VmErrorKind::IntegerToValueCoercionError { from, to } => {
                write!(f, "Failed to convert integer `{from}` to value `{to}`")
            }
            VmErrorKind::ExpectedTupleLength { actual, expected } => write!(
                f,
                "Expected a tuple of length `{expected}`, but found one with length `{actual}`",
            ),
            VmErrorKind::ExpectedVecLength { actual, expected } => write!(
                f,
                "Expected a vector of length `{expected}`, but found one with length `{actual}`",
            ),
            VmErrorKind::ConstNotSupported { actual } => {
                write!(f, "Type `{actual}` can't be converted to a constant value")
            }
            VmErrorKind::MissingInterfaceEnvironment => {
                write!(f, "Missing interface environment")
            }
            VmErrorKind::ExpectedExitedExecutionState { actual } => {
                write!(f, "Expected exited execution state, but was {actual}")
            }
            VmErrorKind::GeneratorComplete => {
                write!(f, "Cannot resume a generator that has completed")
            }
            VmErrorKind::FutureCompleted => write!(f, "Future already completed"),
            VmErrorKind::MissingVariant { name } => write!(f, "No variant matching `{name}`"),
            VmErrorKind::MissingField { target, field } => {
                write!(f, "Missing field `{field}` on `{target}`")
            }
            VmErrorKind::MissingVariantName => {
                write!(f, "missing variant name in runtime information")
            }
            VmErrorKind::MissingStructField { target, name } => write!(
                f,
                "missing dynamic field for struct field `{target}::{name}`",
            ),
            VmErrorKind::MissingTupleIndex { target, index } => write!(
                f,
                "missing dynamic index #{index} in tuple struct `{target}`",
            ),
            VmErrorKind::ExpectedVariant { actual } => {
                write!(f, "Expected an enum variant, but got `{actual}`")
            }
            VmErrorKind::UnsupportedObjectFieldGet { target } => write!(
                f,
                "The object field get operation is not supported on `{target}`",
            ),
            VmErrorKind::IllegalFloatComparison { lhs, rhs } => {
                write!(
                    f,
                    "Cannot perform a comparison of the floats {lhs} and {rhs}",
                )
            }
            VmErrorKind::IllegalFloatOperation { value } => {
                write!(f, "Cannot perform operation on float `{value}`")
            }
            VmErrorKind::MissingCallFrame => {
                write!(f, "Missing call frame for internal vm call")
            }
            VmErrorKind::IllegalFormat => {
                write!(f, "Value cannot be formatted")
            }
        }
    }
}

impl From<RuntimeError> for VmErrorKind {
    #[inline]
    fn from(value: RuntimeError) -> Self {
        value.into_vm_error_kind()
    }
}

impl From<AnySequenceTakeError> for VmErrorKind {
    #[inline]
    fn from(value: AnySequenceTakeError) -> Self {
        match value {
            AnySequenceTakeError::Access(error) => Self::from(error),
            AnySequenceTakeError::Alloc(error) => Self::from(error),
        }
    }
}

impl From<AnyObjError> for VmErrorKind {
    #[inline]
    fn from(error: AnyObjError) -> Self {
        Self::from(RuntimeError::from(error))
    }
}

impl From<Infallible> for VmErrorKind {
    #[inline]
    fn from(error: Infallible) -> Self {
        match error {}
    }
}

impl From<AccessError> for VmErrorKind {
    #[inline]
    fn from(error: AccessError) -> Self {
        VmErrorKind::AccessError { error }
    }
}

impl From<StackError> for VmErrorKind {
    #[inline]
    fn from(error: StackError) -> Self {
        VmErrorKind::StackError { error }
    }
}

impl From<SliceError> for VmErrorKind {
    #[inline]
    fn from(error: SliceError) -> Self {
        VmErrorKind::SliceError { error }
    }
}

impl From<BadInstruction> for VmErrorKind {
    #[inline]
    fn from(error: BadInstruction) -> Self {
        VmErrorKind::BadInstruction { error }
    }
}

impl From<BadJump> for VmErrorKind {
    #[inline]
    fn from(error: BadJump) -> Self {
        VmErrorKind::BadJump { error }
    }
}

impl From<DynArgsUsed> for VmErrorKind {
    #[inline]
    fn from(error: DynArgsUsed) -> Self {
        VmErrorKind::DynArgsUsed { error }
    }
}

impl From<alloc::Error> for VmErrorKind {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        VmErrorKind::AllocError { error }
    }
}

impl From<alloc::alloc::AllocError> for VmErrorKind {
    #[inline]
    fn from(error: alloc::alloc::AllocError) -> Self {
        VmErrorKind::AllocError {
            error: error.into(),
        }
    }
}

impl VmErrorKind {
    /// Bad argument.
    pub(crate) fn bad_argument(arg: usize) -> Self {
        Self::BadArgument { arg }
    }

    /// Construct an expected error.
    pub(crate) fn expected<T>(actual: TypeInfo) -> Self
    where
        T: ?Sized + TypeOf,
    {
        Self::ExpectedType {
            expected: T::type_info(),
            actual,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq))]
enum VmIntegerReprKind {
    Signed(i128),
    Unsigned(u128),
    Isize(isize),
    Usize(usize),
}

/// A type-erased integer representation.
#[derive(Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) struct VmIntegerRepr {
    kind: VmIntegerReprKind,
}

impl VmIntegerRepr {
    #[inline]
    fn new(kind: VmIntegerReprKind) -> Self {
        Self { kind }
    }
}

macro_rules! impl_from {
    ($($variant:ident => [$($ty:ty),* $(,)?]),* $(,)?) => {
        $($(
            impl From<$ty> for VmIntegerRepr {
                #[inline]
                fn from(value: $ty) -> Self {
                    Self::new(VmIntegerReprKind::$variant(From::from(value)))
                }
            }
        )*)*
    };
}

impl_from! {
    Signed => [i8, i16, i32, i64, i128],
    Unsigned => [u8, u16, u32, u64, u128],
    Isize => [isize],
    Usize => [usize],
}

impl fmt::Display for VmIntegerRepr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            VmIntegerReprKind::Signed(value) => value.fmt(f),
            VmIntegerReprKind::Unsigned(value) => value.fmt(f),
            VmIntegerReprKind::Isize(value) => value.fmt(f),
            VmIntegerReprKind::Usize(value) => value.fmt(f),
        }
    }
}

impl fmt::Debug for VmIntegerRepr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}
