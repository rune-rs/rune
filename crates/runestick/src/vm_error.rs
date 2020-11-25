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

    /// Access the underlying error kind while consuming the error.
    pub fn into_kind(self) -> VmErrorKind {
        *self.kind
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
    pub fn as_unwound<'a>(&'a self) -> (&'a VmErrorKind, Option<(&'a Arc<Unit>, usize)>) {
        match &*self.kind {
            VmErrorKind::Unwound { kind, unit, ip } => (&*kind, Some((unit, *ip))),
            kind => (kind, None),
        }
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
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum VmErrorKind {
    /// A vm error that was propagated from somewhere else.
    ///
    /// In order to represent this, we need to preserve the instruction pointer
    /// and eventually unit from where the error happened.
    #[error("{kind} (at inst {ip})")]
    Unwound {
        /// The wrapper error.
        kind: Box<VmErrorKind>,
        /// Associated unit.
        unit: Arc<Unit>,
        /// The instruction pointer of where the original error happened.
        ip: usize,
    },
    #[error("{error}")]
    AccessError {
        #[from]
        error: AccessError,
    },
    #[error("panicked: {reason}")]
    Panic { reason: Panic },
    #[error("no running virtual machines")]
    NoRunningVm,
    #[error("halted for unexpected reason `{halt}`")]
    Halted { halt: VmHaltInfo },
    #[error("failed to format argument")]
    FormatError,
    #[error("stack error: {error}")]
    StackError {
        #[from]
        error: StackError,
    },
    #[error("numerical overflow")]
    Overflow,
    #[error("numerical underflow")]
    Underflow,
    #[error("division by zero")]
    DivideByZero,
    #[error("missing constant with hash `{hash}`")]
    MissingConst { hash: Hash },
    #[error("missing entry `{item}` with hash `{hash}`")]
    MissingEntry { item: Item, hash: Hash },
    #[error("missing function with hash `{hash}`")]
    MissingFunction { hash: Hash },
    #[error("missing instance function `{hash}` for `{instance}`")]
    MissingInstanceFunction { hash: Hash, instance: TypeInfo },
    #[error("instruction pointer is out-of-bounds")]
    IpOutOfBounds,
    #[error("unsupported target for .await `{actual}`")]
    UnsupportedAwait { actual: TypeInfo },
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
    BadArgumentType {
        arg: usize,
        expected: TypeInfo,
        actual: TypeInfo,
    },
    #[error("bad argument #{arg}: {error}")]
    BadArgument {
        #[source]
        error: VmError,
        arg: usize,
    },
    #[error("the index set operation `{target}[{index}] = {value}` is not supported")]
    UnsupportedIndexSet {
        target: TypeInfo,
        index: TypeInfo,
        value: TypeInfo,
    },
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
    #[error("missing index by static string slot `{slot}` in object")]
    ObjectIndexMissing { slot: usize },
    #[error("missing index `{index}` on `{target}`")]
    MissingIndex {
        target: TypeInfo,
        index: VmIntegerRepr,
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
    #[error("expected result or option as value, but got `{actual}`")]
    UnsupportedIsValueOperand { actual: TypeInfo },
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
    #[error("missing interface environment")]
    MissingInterfaceEnvironment,
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
