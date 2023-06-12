use core::fmt;

use crate::no_std as std;
use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;
use crate::no_std::thiserror;

use thiserror::Error;

use crate::compile::ItemBuf;
use crate::hash::Hash;
use crate::runtime::unit::{BadInstruction, BadJump};
use crate::runtime::{
    AccessError, BoxedPanic, CallFrame, ExecutionState, FullTypeOf, Key, MaybeTypeOf, Panic,
    StackError, TypeInfo, TypeOf, Unit, Value, Vm, VmHaltInfo,
};

/// Trait used to convert result types to [`VmResult`].
#[doc(hidden)]
pub trait TryFromResult {
    /// The ok type produced by the conversion.
    type Ok;

    /// The conversion method itself.
    fn try_from_result(value: Self) -> VmResult<Self::Ok>;
}

/// Helper to coerce one result type into [`VmResult`].
///
/// Despite being public, this is actually private API (`#[doc(hidden)]`). Use
/// at your own risk.
#[doc(hidden)]
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
pub struct VmErrorAt {
    /// The instruction which caused the error.
    instruction: usize,
    /// The kind of error.
    kind: VmErrorKind,
}

impl VmErrorAt {
    /// Get the instruction which caused the error.
    pub fn instruction(&self) -> usize {
        self.instruction
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
    pub(crate) chain: Vec<VmErrorAt>,
    pub(crate) stacktrace: Vec<VmErrorLocation>,
}

/// A virtual machine error which includes tracing information.
pub struct VmError {
    pub(crate) inner: Box<VmErrorInner>,
}

impl VmError {
    /// Construct an error containing a panic.
    pub fn panic<D>(message: D) -> Self
    where
        D: 'static + BoxedPanic,
    {
        Self::from(Panic::custom(message))
    }

    /// Get the location where the error happened.
    pub fn at(&self) -> &VmErrorAt {
        &self.inner.error
    }

    /// Get the full backtrace of errors and their corresponding instructions.
    pub fn chain(&self) -> &[VmErrorAt] {
        &self.inner.chain
    }

    /// Construct an expectation error. The actual type received is `actual`,
    /// but we expected `E`.
    pub fn expected<E>(actual: TypeInfo) -> Self
    where
        E: TypeOf,
    {
        Self::from(VmErrorKind::Expected {
            expected: E::type_info(),
            actual,
        })
    }

    /// Construct an overflow error.
    pub fn overflow() -> Self {
        Self::from(VmErrorKind::Overflow)
    }

    /// Get the first error location.
    pub fn first_location(&self) -> Option<&VmErrorLocation> {
        self.inner.stacktrace.first()
    }

    #[cfg(test)]
    pub(crate) fn into_kind(self) -> VmErrorKind {
        self.inner.error.kind
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

impl crate::no_std::error::Error for VmError {}

/// A result produced by the virtual machine.
#[must_use]
pub enum VmResult<T> {
    /// A produced value.
    Ok(T),
    /// Multiple errors with locations included.
    Err(VmError),
}

impl<T> VmResult<T> {
    /// Construct a result containing a panic.
    pub fn panic<D>(message: D) -> Self
    where
        D: 'static + BoxedPanic,
    {
        Self::err(Panic::custom(message))
    }

    /// Construct an expectation error. The actual type received is `actual`,
    /// but we expected `E`.
    pub fn expected<E>(actual: TypeInfo) -> Self
    where
        E: TypeOf,
    {
        Self::Err(VmError::expected::<E>(actual))
    }

    /// Construct a new error from a type that can be converted into a
    /// [`VmError`].
    pub(crate) fn err<E>(error: E) -> Self
    where
        VmErrorKind: From<E>,
    {
        Self::Err(VmError::from(error))
    }

    /// Convert a [`VmResult`] into a [`Result`].
    #[inline(always)]
    pub fn into_result(self) -> Result<T, VmError> {
        match self {
            Self::Ok(value) => Ok(value),
            Self::Err(error) => Err(error),
        }
    }

    /// Apply the given frame to the current result.
    pub(crate) fn with_vm(self, vm: &Vm) -> Self {
        match self {
            Self::Ok(ok) => Self::Ok(ok),
            Self::Err(mut err) => {
                err.inner.stacktrace.push(VmErrorLocation {
                    unit: vm.unit().clone(),
                    ip: vm.ip(),
                    frames: vm.call_frames().to_vec(),
                });

                Self::Err(err)
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
            Self::Ok(ok) => Self::Ok(ok),
            Self::Err(mut err) => {
                let index = err.inner.stacktrace.len();

                err.inner.chain.push(VmErrorAt {
                    instruction: index,
                    kind: VmErrorKind::from(error()),
                });

                Self::Err(err)
            }
        }
    }

    /// Expect a value or panic.
    #[doc(hidden)]
    pub fn expect(self, msg: &str) -> T {
        self.into_result().expect(msg)
    }

    /// Unwrap the interior value.
    #[doc(hidden)]
    pub fn unwrap(self) -> T {
        self.into_result().unwrap()
    }

    /// Test if it is an error.
    #[doc(hidden)]
    pub fn is_err(&self) -> bool {
        matches!(self, Self::Err(..))
    }
}

#[allow(non_snake_case)]
impl<T> VmResult<T> {
    #[doc(hidden)]
    #[inline]
    pub fn __rune_macros__missing_struct_field(target: &'static str, name: &'static str) -> Self {
        Self::err(VmErrorKind::MissingStructField { target, name })
    }

    #[doc(hidden)]
    #[inline]
    pub fn __rune_macros__missing_variant(name: &str) -> Self {
        Self::err(VmErrorKind::MissingVariant {
            name: name.to_owned(),
        })
    }

    #[doc(hidden)]
    #[inline]
    pub fn __rune_macros__expected_variant(actual: TypeInfo) -> Self {
        Self::err(VmErrorKind::ExpectedVariant { actual })
    }

    #[doc(hidden)]
    #[inline]
    pub fn __rune_macros__missing_variant_name() -> Self {
        Self::err(VmErrorKind::MissingVariantName)
    }

    #[doc(hidden)]
    #[inline]
    pub fn __rune_macros__missing_tuple_index(target: &'static str, index: usize) -> Self {
        Self::err(VmErrorKind::MissingTupleIndex { target, index })
    }

    #[doc(hidden)]
    #[inline]
    pub fn __rune_macros__unsupported_object_field_get(target: TypeInfo) -> Self {
        Self::err(VmErrorKind::UnsupportedObjectFieldGet { target })
    }

    #[doc(hidden)]
    #[inline]
    pub fn __rune_macros__unsupported_tuple_index_get(target: TypeInfo) -> Self {
        Self::err(VmErrorKind::UnsupportedTupleIndexGet { target })
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

#[cfg(feature = "std")]
impl<T> ::std::process::Termination for VmResult<T> {
    #[inline]
    fn report(self) -> ::std::process::ExitCode {
        match self {
            VmResult::Ok(_) => ::std::process::ExitCode::SUCCESS,
            VmResult::Err(_) => ::std::process::ExitCode::FAILURE,
        }
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
                    instruction: 0,
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
#[derive(Debug, Error)]
#[non_exhaustive]
#[doc(hidden)]
pub(crate) enum VmErrorKind {
    #[error("{error}")]
    AccessError {
        #[from]
        error: AccessError,
    },
    #[error("Stack error: {error}")]
    StackError {
        #[from]
        error: StackError,
    },
    #[error("{error}")]
    BadInstruction {
        #[from]
        error: BadInstruction,
    },
    #[error("{error}")]
    BadJump {
        #[from]
        error: BadJump,
    },
    #[error("Panicked: {reason}")]
    Panic { reason: Panic },
    #[error("No running virtual machines")]
    NoRunningVm,
    #[error("Halted for unexpected reason `{halt}`")]
    Halted { halt: VmHaltInfo },
    #[error("Failed to format argument")]
    FormatError,
    #[error("Numerical overflow")]
    Overflow,
    #[error("Numerical underflow")]
    Underflow,
    #[error("Division by zero")]
    DivideByZero,
    #[error("Missing entry `{item}` with hash `{hash}`")]
    MissingEntry { item: ItemBuf, hash: Hash },
    #[error("Missing entry with hash `{hash}`")]
    MissingEntryHash { hash: Hash },
    #[error("Missing function with hash `{hash}`")]
    MissingFunction { hash: Hash },
    #[error("Missing context function with hash `{hash}`")]
    MissingContextFunction { hash: Hash },
    #[error("Missing instance function `{hash}` for `{instance}`")]
    MissingInstanceFunction { hash: Hash, instance: TypeInfo },
    #[error("Instruction pointer `{ip}` is out-of-bounds `0-{length}`")]
    IpOutOfBounds { ip: usize, length: usize },
    #[error("Unsupported operation `{lhs} {op} {rhs}`")]
    UnsupportedBinaryOperation {
        op: &'static str,
        lhs: TypeInfo,
        rhs: TypeInfo,
    },
    #[error("Unsupported operation `{op}{operand}`")]
    UnsupportedUnaryOperation { op: &'static str, operand: TypeInfo },
    #[error("Static string slot `{slot}` does not exist")]
    MissingStaticString { slot: usize },
    #[error("Static object keys slot `{slot}` does not exist")]
    MissingStaticObjectKeys { slot: usize },
    #[error("Missing runtime information for variant with hash `{hash}`")]
    MissingVariantRtti { hash: Hash },
    #[error("Missing runtime information for type with hash `{hash}`")]
    MissingRtti { hash: Hash },
    #[error("Wrong number of arguments `{actual}`, expected `{expected}`")]
    BadArgumentCount { actual: usize, expected: usize },
    #[error("Bad argument #{arg}, expected `{expected}` but got `{actual}`")]
    BadArgumentAt {
        arg: usize,
        expected: TypeInfo,
        actual: TypeInfo,
    },
    #[error("Bad argument at #{arg}")]
    BadArgument { arg: usize },
    #[error("The index set operation `{target}[{index}] = {value}` is not supported")]
    UnsupportedIndexSet {
        target: TypeInfo,
        index: TypeInfo,
        value: TypeInfo,
    },
    #[error("The index get operation `{target}[{index}]` is not supported")]
    UnsupportedIndexGet { target: TypeInfo, index: TypeInfo },
    #[error("The tuple index get operation is not supported on `{target}`")]
    UnsupportedTupleIndexGet { target: TypeInfo },
    #[error("The tuple index set operation is not supported on `{target}`")]
    UnsupportedTupleIndexSet { target: TypeInfo },
    #[error("Field not available on `{target}`")]
    UnsupportedObjectSlotIndexGet { target: TypeInfo },
    #[error("Field not available on `{target}`")]
    UnsupportedObjectSlotIndexSet { target: TypeInfo },
    #[error("Operation `{value} is {test_type}` is not supported")]
    UnsupportedIs {
        value: TypeInfo,
        test_type: TypeInfo,
    },
    #[error("Type `{actual}` cannot be called since it's not a function")]
    UnsupportedCallFn { actual: TypeInfo },
    #[error("Missing index by static string slot `{slot}`")]
    ObjectIndexMissing { slot: usize },
    #[error("Type `{target}` missing index `{index}`")]
    MissingIndex {
        target: TypeInfo,
        index: VmIntegerRepr,
    },
    #[error("Type `{target}` missing index `{index:?}`")]
    MissingIndexKey { target: TypeInfo, index: Key },
    #[error("Index out of bounds, the length is `{length}` but the index is `{index}`")]
    OutOfRange {
        index: VmIntegerRepr,
        length: VmIntegerRepr,
    },
    #[error("Type `{actual}` is not supported as try operand")]
    UnsupportedTryOperand { actual: TypeInfo },
    #[error("Type `{actual}` is not supported as iter-next operand")]
    UnsupportedIterNextOperand { actual: TypeInfo },
    #[error("Expected type `{expected}`, but found `{actual}`")]
    Expected {
        expected: TypeInfo,
        actual: TypeInfo,
    },
    #[error("Expected `Any` type, but found `{actual}`")]
    ExpectedAny { actual: TypeInfo },
    #[error("Failed to convert value `{from}` to integer `{to}`")]
    ValueToIntegerCoercionError {
        from: VmIntegerRepr,
        to: &'static str,
    },
    #[error("Failed to convert integer `{from}` to value `{to}`")]
    IntegerToValueCoercionError {
        from: VmIntegerRepr,
        to: &'static str,
    },
    #[error("Expected a tuple of length `{expected}`, but found one with length `{actual}`")]
    ExpectedTupleLength { actual: usize, expected: usize },
    #[error("Type `{actual}` can't be converted to a constant value")]
    ConstNotSupported { actual: TypeInfo },
    #[error("Type `{actual}` can't be converted to a hash key")]
    KeyNotSupported { actual: TypeInfo },
    #[error("Missing interface environment")]
    MissingInterfaceEnvironment,
    #[error("Unsupported range")]
    UnsupportedRange,
    #[error("Expected execution to be {expected}, but was {actual}")]
    ExpectedExecutionState {
        expected: ExecutionState,
        actual: ExecutionState,
    },
    #[error("Cannot resume a generator that has completed")]
    GeneratorComplete,
    #[error("Future already completed")]
    FutureCompleted,
    // Used in rune-macros.
    #[error("No variant matching `{name}`")]
    MissingVariant { name: String },
    #[error("Missing field `{field}` on `{target}`")]
    MissingField { target: TypeInfo, field: String },
    #[error("missing variant name in runtime information")]
    MissingVariantName,
    #[error("missing dynamic field for struct field `{target}::{name}`")]
    MissingStructField {
        target: &'static str,
        name: &'static str,
    },
    #[error("missing dynamic index #{index} in tuple struct `{target}`")]
    MissingTupleIndex { target: &'static str, index: usize },
    #[error("Expected an enum variant, but got `{actual}`")]
    ExpectedVariant { actual: TypeInfo },
    #[error("The object field get operation is not supported on `{target}`")]
    UnsupportedObjectFieldGet { target: TypeInfo },
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
pub struct VmIntegerRepr(num::BigInt);

impl<T> From<T> for VmIntegerRepr
where
    num::BigInt: From<T>,
{
    fn from(value: T) -> Self {
        Self(num::BigInt::from(value))
    }
}

impl fmt::Display for VmIntegerRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
