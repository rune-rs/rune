use crate::compile::ItemBuf;
use crate::hash::Hash;
use crate::runtime::{
    AccessError, CallFrame, ExecutionState, FullTypeOf, Key, MaybeTypeOf, Panic, Protocol,
    StackError, TypeInfo, TypeOf, Unit, Value, Vm, VmHaltInfo,
};
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

/// Trait used to convert result types to [`VmResult`].
#[doc(hidden)]
pub trait TryFromResult {
    /// The ok type produced by the conversion.
    type Ok;

    /// The conversion method itself.
    fn try_from_result(value: Self) -> VmResult<Self::Ok>;
}

/// Helper to coerce one result type into [`VmResult`].
pub fn try_result<T>(result: T) -> VmResult<T::Ok>
where
    T: TryFromResult,
{
    T::try_from_result(result)
}

impl<T> TryFromResult for VmResult<T> {
    type Ok = T;

    #[inline]
    fn try_from_result(value: Self) -> VmResult<T> {
        value
    }
}

impl<T, E> TryFromResult for Result<T, E>
where
    VmErrorKind: From<E>,
{
    type Ok = T;

    #[inline]
    fn try_from_result(value: Self) -> VmResult<T> {
        match value {
            Ok(ok) => VmResult::Ok(ok),
            Err(err) => VmResult::err(err),
        }
    }
}

impl<T> TryFromResult for Result<T, VmError> {
    type Ok = T;

    #[inline]
    fn try_from_result(value: Self) -> VmResult<T> {
        match value {
            Ok(ok) => VmResult::Ok(ok),
            Err(err) => VmResult::Err(err),
        }
    }
}

/// A single unit producing errors.
#[derive(Debug)]
#[non_exhaustive]
pub struct VmErrorLocation {
    /// Associated unit.
    pub unit: Arc<Unit>,
    /// Frozen instruction pointer.
    pub ip: usize,
    /// All lower call frames before the unwind trigger point
    pub frames: Vec<CallFrame>,
}

#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct VmErrorAt {
    pub(crate) index: usize,
    pub(crate) kind: VmErrorKind,
}

#[non_exhaustive]
pub(crate) struct VmErrorInner {
    pub(crate) error: VmErrorAt,
    pub(crate) chain: Vec<VmErrorAt>,
    pub(crate) stacktrace: Vec<VmErrorLocation>,
}

/// A virtual machine error which includes tracing information.
pub struct VmError {
    pub(crate) inner: Box<VmErrorInner>,
}

impl VmError {
    /// Get the first error location.
    pub fn first_location(&self) -> Option<&VmErrorLocation> {
        self.inner.stacktrace.first()
    }

    /// Access the underlying error kind.
    pub fn kind(&self) -> &VmErrorKind {
        &self.inner.error.kind
    }

    /// Access the underlying error kind while consuming the error.
    pub fn into_kind(self) -> VmErrorKind {
        self.inner.error.kind
    }
}

impl fmt::Display for VmError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.error.kind.fmt(f)
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

impl std::error::Error for VmError {}

/// A result produced by the virtual machine.
#[must_use]
pub enum VmResult<T> {
    /// A produced value.
    Ok(T),
    /// Multiple errors with locations included.
    Err(VmError),
}

impl<T> VmResult<T> {
    /// Construct a new error from a type that can be converted into a
    /// [`VmError`].
    pub fn err<E>(error: E) -> Self
    where
        VmErrorKind: From<E>,
    {
        VmResult::Err(VmError::from(error))
    }

    /// Convert a [`VmResult`] into a [`Result`].
    #[inline(always)]
    pub fn into_result(self) -> Result<T, VmError> {
        match self {
            VmResult::Ok(value) => Ok(value),
            VmResult::Err(error) => Err(error),
        }
    }

    /// Apply the given frame to the current result.
    pub(crate) fn with_vm(self, vm: &Vm) -> Self {
        match self {
            VmResult::Ok(ok) => VmResult::Ok(ok),
            VmResult::Err(mut err) => {
                err.inner.stacktrace.push(VmErrorLocation {
                    unit: vm.unit().clone(),
                    ip: vm.ip(),
                    frames: vm.call_frames().to_vec(),
                });

                VmResult::Err(err)
            }
        }
    }

    /// Add auxilliary errors if appropriate.
    #[inline]
    pub(crate) fn with_error<E, O>(self, error: E) -> Self
    where
        E: FnOnce() -> O,
        VmErrorKind: From<O>,
    {
        match self {
            VmResult::Ok(ok) => VmResult::Ok(ok),
            VmResult::Err(mut err) => {
                let index = err.inner.stacktrace.len();

                err.inner.chain.push(VmErrorAt {
                    index,
                    kind: VmErrorKind::from(error()),
                });

                VmResult::Err(err)
            }
        }
    }

    /// Expect a value or panic.
    pub fn expect(self, msg: &str) -> T {
        match self {
            VmResult::Ok(t) => t,
            VmResult::Err(error) => panic!("{msg}: {error:?}"),
        }
    }
}

impl<T> MaybeTypeOf for VmResult<T>
where
    T: MaybeTypeOf,
{
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        T::maybe_type_of()
    }
}

impl<E> From<E> for VmError
where
    VmErrorKind: From<E>,
{
    fn from(error: E) -> Self {
        Self {
            inner: Box::new(VmErrorInner {
                error: VmErrorAt {
                    index: 0,
                    kind: VmErrorKind::from(error),
                },
                chain: Vec::new(),
                stacktrace: Vec::new(),
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

/// The kind of error encountered.
#[allow(missing_docs)]
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VmErrorKind {
    #[error("{error}")]
    AccessError {
        #[from]
        error: AccessError,
    },
    #[error("stack error: {error}")]
    StackError {
        #[from]
        error: StackError,
    },
    #[error("panicked: {reason}")]
    Panic { reason: Panic },
    #[error("no running virtual machines")]
    NoRunningVm,
    #[error("halted for unexpected reason `{halt}`")]
    Halted { halt: VmHaltInfo },
    #[error("failed to format argument")]
    FormatError,
    #[error("numerical overflow")]
    Overflow,
    #[error("numerical underflow")]
    Underflow,
    #[error("division by zero")]
    DivideByZero,
    #[error("missing constant with hash `{hash}`")]
    MissingConst { hash: Hash },
    #[error("missing entry `{item}` with hash `{hash}`")]
    MissingEntry { item: ItemBuf, hash: Hash },
    #[error("missing entry with hash `{hash}`")]
    MissingEntryHash { hash: Hash },
    #[error("missing function with hash `{hash}`")]
    MissingFunction { hash: Hash },
    #[error("missing instance function `{hash}` for `{instance}`")]
    MissingInstanceFunction { hash: Hash, instance: TypeInfo },
    #[error("instruction pointer is out-of-bounds")]
    IpOutOfBounds,
    #[error("unsupported vm operation `{lhs} {op} {rhs}`")]
    UnsupportedBinaryOperation {
        op: &'static str,
        lhs: TypeInfo,
        rhs: TypeInfo,
    },
    #[error("unsupported vm operation `{op}{operand}`")]
    UnsupportedUnaryOperation { op: &'static str, operand: TypeInfo },
    #[error("`{actual}` does not implement the `{protocol}` protocol")]
    MissingProtocol {
        protocol: Protocol,
        actual: TypeInfo,
    },
    #[error("static string slot `{slot}` does not exist")]
    MissingStaticString { slot: usize },
    #[error("static object keys slot `{slot}` does not exist")]
    MissingStaticObjectKeys { slot: usize },
    #[error("missing runtime information for variant with hash `{hash}`")]
    MissingVariantRtti { hash: Hash },
    #[error("missing runtime information for type with hash `{hash}`")]
    MissingRtti { hash: Hash },
    #[error("wrong number of arguments `{actual}`, expected `{expected}`")]
    BadArgumentCount { actual: usize, expected: usize },
    #[error("bad argument #{arg}, expected `{expected}` but got `{actual}`")]
    BadArgumentAt {
        arg: usize,
        expected: TypeInfo,
        actual: TypeInfo,
    },
    #[error("bad argument at #{arg}")]
    BadArgument { arg: usize },
    #[error("the index set operation `{target}[{index}] = {value}` is not supported")]
    UnsupportedIndexSet {
        target: TypeInfo,
        index: TypeInfo,
        value: TypeInfo,
    },
    #[error("the object field get operation is not supported on `{target}`")]
    UnsupportedObjectFieldGet { target: TypeInfo },
    #[error("the index get operation `{target}[{index}]` is not supported")]
    UnsupportedIndexGet { target: TypeInfo, index: TypeInfo },
    #[error("the tuple index get operation is not supported on `{target}`")]
    UnsupportedTupleIndexGet { target: TypeInfo },
    #[error("the tuple index set operation is not supported on `{target}`")]
    UnsupportedTupleIndexSet { target: TypeInfo },
    #[error("field not available on `{target}`")]
    UnsupportedObjectSlotIndexGet { target: TypeInfo },
    #[error("field not available on `{target}`")]
    UnsupportedObjectSlotIndexSet { target: TypeInfo },
    #[error("`{value} is {test_type}` is not supported")]
    UnsupportedIs {
        value: TypeInfo,
        test_type: TypeInfo,
    },
    #[error("`{actual_type}` cannot be called since it's not a function")]
    UnsupportedCallFn { actual_type: TypeInfo },
    #[error("missing index by static string slot `{slot}`")]
    ObjectIndexMissing { slot: usize },
    #[error("`{target}` missing index `{index}`")]
    MissingIndex {
        target: TypeInfo,
        index: VmIntegerRepr,
    },
    #[error("`{target}` missing index `{index:?}`")]
    MissingIndexKey { target: TypeInfo, index: Key },
    #[error("index out of bounds: the len is ${len} but the index is {index}")]
    OutOfRange {
        index: VmIntegerRepr,
        len: VmIntegerRepr,
    },
    #[error("missing field `{field}` on `{target}`")]
    MissingField { target: TypeInfo, field: String },
    #[error("missing dynamic field for struct field `{target}::{name}`")]
    MissingStructField {
        target: &'static str,
        name: &'static str,
    },
    #[error("missing dynamic index #{index} in tuple struct `{target}`")]
    MissingTupleIndex { target: &'static str, index: usize },
    #[error("expected result or option with value to unwrap, but got `{actual}`")]
    UnsupportedUnwrap { actual: TypeInfo },
    #[error("expected Some value, but got `None`")]
    UnsupportedUnwrapNone,
    #[error("expected Ok value, but got `Err({err})`")]
    UnsupportedUnwrapErr { err: TypeInfo },
    #[error("value `{actual}` is not supported as try operand")]
    UnsupportedTryOperand { actual: TypeInfo },
    #[error("value `{actual}` is not supported as iter-next operand")]
    UnsupportedIterNextOperand { actual: TypeInfo },
    /// Trying to resume a generator that has completed.
    #[error("cannot resume a generator that has completed")]
    GeneratorComplete,
    #[error("expected `{expected}`, but found `{actual}`")]
    Expected {
        expected: TypeInfo,
        actual: TypeInfo,
    },
    #[error("expected `Any` type, but found `{actual}`")]
    ExpectedAny { actual: TypeInfo },
    #[error("failed to convert value `{from}` to integer `{to}`")]
    ValueToIntegerCoercionError {
        from: VmIntegerRepr,
        to: &'static str,
    },
    #[error("failed to convert integer `{from}` to value `{to}`")]
    IntegerToValueCoercionError {
        from: VmIntegerRepr,
        to: &'static str,
    },
    #[error("expected a tuple of length `{expected}`, but found one with length `{actual}`")]
    ExpectedTupleLength { actual: usize, expected: usize },
    #[error("unexpectedly ran out of items to iterate over")]
    IterationError,
    #[error("missing variant name in runtime information")]
    MissingVariantName,
    #[error("no variant matching `{name}`")]
    MissingVariant { name: Box<str> },
    #[error("expected an enum variant, but got `{actual}`")]
    ExpectedVariant { actual: TypeInfo },
    #[error("{actual} can't be converted to a constant value")]
    ConstNotSupported { actual: TypeInfo },
    #[error("{actual} can't be converted to a hash key")]
    KeyNotSupported { actual: TypeInfo },
    #[error("missing interface environment")]
    MissingInterfaceEnvironment,
    #[error("index out of bounds")]
    IndexOutOfBounds,
    #[error("unsupported range")]
    UnsupportedRange,
    #[error("expected execution to be {expected}, but was {actual}")]
    ExpectedExecutionState {
        expected: ExecutionState,
        actual: ExecutionState,
    },
    #[error("future already completed")]
    FutureCompleted,
}

impl VmErrorKind {
    /// Bad argument.
    pub fn bad_argument<T>(arg: usize, value: &Value) -> VmResult<Self>
    where
        T: TypeOf,
    {
        VmResult::Ok(Self::BadArgumentAt {
            arg,
            expected: T::type_info(),
            actual: vm_try!(value.type_info()),
        })
    }

    /// Construct an expected error.
    pub fn expected<T>(actual: TypeInfo) -> Self
    where
        T: TypeOf,
    {
        Self::Expected {
            expected: T::type_info(),
            actual,
        }
    }

    /// Construct an expected any error.
    pub fn expected_any(actual: TypeInfo) -> Self {
        Self::ExpectedAny { actual }
    }
}

/// A type-erased rust number.
#[derive(Debug, Clone)]
pub struct VmIntegerRepr(num_bigint::BigInt);

impl<T> From<T> for VmIntegerRepr
where
    num_bigint::BigInt: From<T>,
{
    fn from(value: T) -> Self {
        Self(num_bigint::BigInt::from(value))
    }
}

impl fmt::Display for VmIntegerRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
