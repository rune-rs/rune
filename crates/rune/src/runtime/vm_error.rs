use core::convert::Infallible;
use core::fmt;

use ::rust_alloc::boxed::Box;
use ::rust_alloc::sync::Arc;

use crate::alloc::error::CustomError;
use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::compile::ItemBuf;
use crate::hash::Hash;
use crate::runtime::unit::{BadInstruction, BadJump};
use crate::runtime::{
    AccessError, BoxedPanic, CallFrame, ExecutionState, FullTypeOf, MaybeTypeOf, Panic, StackError,
    TypeInfo, TypeOf, Unit, Vm, VmHaltInfo,
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
    pub frames: ::rust_alloc::vec::Vec<CallFrame>,
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
    pub(crate) chain: ::rust_alloc::vec::Vec<VmErrorAt>,
    pub(crate) stacktrace: ::rust_alloc::vec::Vec<VmErrorLocation>,
}

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
                chain: ::rust_alloc::vec::Vec::new(),
                stacktrace: ::rust_alloc::vec::Vec::new(),
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

cfg_std! {
    impl std::error::Error for VmError {}
}

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
        E: ?Sized + TypeOf,
    {
        Self::Err(VmError::expected::<E>(actual))
    }

    /// Construct a new error from a type that can be converted into a
    /// [`VmError`].
    pub(crate) fn err<E>(error: E) -> Self
    where
        VmError: From<E>,
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
                    ip: vm.last_ip(),
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
                #[cfg(feature = "emit")]
                let index = err.inner.stacktrace.len();

                err.inner.chain.push(VmErrorAt {
                    #[cfg(feature = "emit")]
                    index,
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
    pub fn __rune_macros__missing_variant(name: &str) -> alloc::Result<Self> {
        Ok(Self::err(VmErrorKind::MissingVariant {
            name: name.try_to_owned()?,
        }))
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
    pub fn __rune_macros__unsupported_tuple_index_get(target: TypeInfo, index: usize) -> Self {
        Self::err(VmErrorKind::UnsupportedTupleIndexGet { target, index })
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

cfg_std! {
    impl<T> ::std::process::Termination for VmResult<T> {
        #[inline]
        fn report(self) -> ::std::process::ExitCode {
            match self {
                VmResult::Ok(_) => ::std::process::ExitCode::SUCCESS,
                VmResult::Err(_) => ::std::process::ExitCode::FAILURE,
            }
        }
    }
}

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

        let first = match it.next() {
            None => VmErrorKind::Panic {
                reason: Panic::custom("Unknown error"),
            },
            Some(first) => first,
        };

        let mut chain = ::rust_alloc::vec::Vec::with_capacity(it.len());

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
                stacktrace: ::rust_alloc::vec::Vec::new(),
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
#[derive(Debug)]
#[non_exhaustive]
#[doc(hidden)]
pub(crate) enum VmErrorKind {
    AccessError {
        error: AccessError,
    },
    StackError {
        error: StackError,
    },
    BadInstruction {
        error: BadInstruction,
    },
    BadJump {
        error: BadJump,
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
    MissingStaticObjectKeys {
        slot: usize,
    },
    MissingVariantRtti {
        hash: Hash,
    },
    MissingRtti {
        hash: Hash,
    },
    BadArgumentCount {
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
    },
    UnsupportedObjectSlotIndexSet {
        target: TypeInfo,
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
    #[cfg(feature = "alloc")]
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
    Expected {
        expected: TypeInfo,
        actual: TypeInfo,
    },
    ExpectedAny {
        actual: TypeInfo,
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
    ConstNotSupported {
        actual: TypeInfo,
    },
    MissingInterfaceEnvironment,
    ExpectedExecutionState {
        expected: ExecutionState,
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
    #[cfg(feature = "alloc")]
    IllegalFloatOperation {
        value: f64,
    },
    MissingCallFrame,
    IllegalFormat,
    AllocError {
        error: alloc::Error,
    },
}

impl fmt::Display for VmErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VmErrorKind::AccessError { error } => {
                write!(f, "{error}")
            }
            VmErrorKind::StackError { error } => write!(f, "Stack error: {error}",),
            VmErrorKind::BadInstruction { error } => {
                write!(f, "{error}")
            }
            VmErrorKind::BadJump { error } => {
                write!(f, "{error}")
            }
            VmErrorKind::Panic { reason } => write!(f, "Panicked: {reason}",),
            VmErrorKind::NoRunningVm {} => write!(f, "No running virtual machines"),
            VmErrorKind::Halted { halt } => write!(f, "Halted for unexpected reason `{halt}`",),
            VmErrorKind::Overflow {} => write!(f, "Numerical overflow"),
            VmErrorKind::Underflow {} => write!(f, "Numerical underflow"),
            VmErrorKind::DivideByZero {} => write!(f, "Division by zero"),
            VmErrorKind::MissingEntry { item, hash } => {
                write!(f, "Missing entry `{item}` with hash `{hash}`",)
            }
            VmErrorKind::MissingEntryHash { hash } => {
                write!(f, "Missing entry with hash `{hash}`",)
            }
            VmErrorKind::MissingFunction { hash } => {
                write!(f, "Missing function with hash `{hash}`",)
            }
            VmErrorKind::MissingContextFunction { hash } => {
                write!(f, "Missing context function with hash `{hash}`",)
            }
            VmErrorKind::MissingInstanceFunction { hash, instance } => {
                write!(f, "Missing instance function `{hash}` for `{instance}`",)
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
                write!(f, "Unsupported unary operation `{op}` on {operand}",)
            }
            VmErrorKind::MissingStaticString { slot } => {
                write!(f, "Static string slot `{slot}` does not exist",)
            }
            VmErrorKind::MissingStaticObjectKeys { slot } => {
                write!(f, "Static object keys slot `{slot}` does not exist",)
            }
            VmErrorKind::MissingVariantRtti { hash } => write!(
                f,
                "Missing runtime information for variant with hash `{hash}`",
            ),
            VmErrorKind::MissingRtti { hash } => {
                write!(f, "Missing runtime information for type with hash `{hash}`",)
            }
            VmErrorKind::BadArgumentCount { actual, expected } => write!(
                f,
                "Wrong number of arguments `{actual}`, expected `{expected}`",
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
            VmErrorKind::UnsupportedObjectSlotIndexGet { target } => {
                write!(f, "Field not available to get on `{target}`",)
            }
            VmErrorKind::UnsupportedObjectSlotIndexSet { target } => {
                write!(f, "Field not available to set on `{target}`",)
            }
            VmErrorKind::UnsupportedIs { value, test_type } => {
                write!(f, "Operation `{value} is {test_type}` is not supported",)
            }
            VmErrorKind::UnsupportedAs { value, type_hash } => {
                write!(f, "Operation `{value} as {type_hash}` is not supported",)
            }
            VmErrorKind::UnsupportedCallFn { actual } => write!(
                f,
                "Type `{actual}` cannot be called since it's not a function",
            ),
            VmErrorKind::ObjectIndexMissing { slot } => {
                write!(f, "Missing index by static string slot `{slot}`",)
            }
            VmErrorKind::MissingIndex { target } => {
                write!(f, "Type `{target}` missing index",)
            }
            VmErrorKind::MissingIndexInteger { target, index } => {
                write!(f, "Type `{target}` missing integer index `{index}`",)
            }
            #[cfg(feature = "alloc")]
            VmErrorKind::MissingIndexKey { target } => {
                write!(f, "Type `{target}` missing index",)
            }
            VmErrorKind::OutOfRange { index, length } => write!(
                f,
                "Index out of bounds, the length is `{length}` but the index is `{index}`",
            ),
            VmErrorKind::UnsupportedTryOperand { actual } => {
                write!(f, "Type `{actual}` is not supported as try operand",)
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
                write!(f, "Type `{actual}` is not supported as iter-next operand",)
            }
            VmErrorKind::Expected { expected, actual } => {
                write!(f, "Expected type `{expected}` but found `{actual}`",)
            }
            VmErrorKind::ExpectedAny { actual } => {
                write!(f, "Expected `Any` type, but found `{actual}`",)
            }
            VmErrorKind::ValueToIntegerCoercionError { from, to } => {
                write!(f, "Failed to convert value `{from}` to integer `{to}`",)
            }
            VmErrorKind::IntegerToValueCoercionError { from, to } => {
                write!(f, "Failed to convert integer `{from}` to value `{to}`",)
            }
            VmErrorKind::ExpectedTupleLength { actual, expected } => write!(
                f,
                "Expected a tuple of length `{expected}`, but found one with length `{actual}`",
            ),
            VmErrorKind::ConstNotSupported { actual } => {
                write!(f, "Type `{actual}` can't be converted to a constant value",)
            }
            VmErrorKind::MissingInterfaceEnvironment {} => {
                write!(f, "Missing interface environment")
            }
            VmErrorKind::ExpectedExecutionState { expected, actual } => {
                write!(f, "Expected execution to be {expected}, but was {actual}",)
            }
            VmErrorKind::GeneratorComplete {} => {
                write!(f, "Cannot resume a generator that has completed")
            }
            VmErrorKind::FutureCompleted {} => write!(f, "Future already completed"),
            VmErrorKind::MissingVariant { name } => write!(f, "No variant matching `{name}`",),
            VmErrorKind::MissingField { target, field } => {
                write!(f, "Missing field `{field}` on `{target}`",)
            }
            VmErrorKind::MissingVariantName {} => {
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
                write!(f, "Expected an enum variant, but got `{actual}`",)
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
            #[cfg(feature = "alloc")]
            VmErrorKind::IllegalFloatOperation { value } => {
                write!(f, "Cannot perform operation on float `{value}`",)
            }
            VmErrorKind::MissingCallFrame => {
                write!(f, "Missing call frame for internal vm call")
            }
            VmErrorKind::IllegalFormat => {
                write!(f, "Value cannot be formatted")
            }
            VmErrorKind::AllocError { error } => error.fmt(f),
        }
    }
}

impl From<Infallible> for VmErrorKind {
    #[inline(always)]
    fn from(error: Infallible) -> Self {
        match error {}
    }
}

impl From<AccessError> for VmErrorKind {
    #[allow(deprecated)]
    fn from(error: AccessError) -> Self {
        VmErrorKind::AccessError { error }
    }
}

impl From<StackError> for VmErrorKind {
    #[allow(deprecated)]
    fn from(error: StackError) -> Self {
        VmErrorKind::StackError { error }
    }
}

impl From<BadInstruction> for VmErrorKind {
    #[allow(deprecated)]
    fn from(error: BadInstruction) -> Self {
        VmErrorKind::BadInstruction { error }
    }
}

impl From<BadJump> for VmErrorKind {
    #[allow(deprecated)]
    fn from(error: BadJump) -> Self {
        VmErrorKind::BadJump { error }
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
    pub fn bad_argument(arg: usize) -> Self {
        Self::BadArgument { arg }
    }

    /// Construct an expected error.
    pub fn expected<T>(actual: TypeInfo) -> Self
    where
        T: ?Sized + TypeOf,
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
