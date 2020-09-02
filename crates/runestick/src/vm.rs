use crate::future::SelectFuture;
use crate::unit::{UnitFnCall, UnitFnKind};
use crate::{
    AccessError, Bytes, Context, FnPtr, FromValue, Future, Generator, Hash, Inst, Integer,
    IntoArgs, IntoTypeHash, Object, Panic, Protocol, Shared, Stack, StackError, StaticString,
    ToValue, Tuple, TypeCheck, TypedObject, TypedTuple, Unit, Value, ValueError, ValueTypeInfo,
    VariantObject, VariantTuple,
};
use std::any;
use std::fmt;
use std::marker;
use std::mem;
use std::rc::Rc;
use thiserror::Error;

/// Errors raised by the execution of the virtual machine.
#[derive(Debug, Error)]
pub enum VmError {
    /// The virtual machine panicked for a specific reason.
    #[error("panicked `{reason}`")]
    Panic {
        /// The reason for the panic.
        reason: Panic,
    },
    /// The virtual machine stopped for an unexpected reason.
    #[error("stopped for unexpected reason `{reason}`")]
    Stopped {
        /// The reason why the virtual machine stopped.
        reason: StopReason,
    },
    /// A vm error that was propagated from somewhere else.
    ///
    /// In order to represent this, we need to preserve the instruction pointer
    /// and eventually unit from where the error happened.
    #[error("{error} (at {ip})")]
    UnwindedVmError {
        /// The actual error.
        error: Box<VmError>,
        /// The instruction pointer of where the original error happened.
        ip: usize,
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
    /// Trying to access an inaccessible reference.
    #[error("failed to access value: {error}")]
    AccessError {
        /// Source error.
        #[from]
        error: AccessError,
    },
    /// Error raised when trying to access a value.
    #[error("value error: {error}")]
    ValueError {
        /// Source error.
        #[source]
        error: ValueError,
    },
    /// The virtual machine panicked for a specific reason.
    #[error("panicked `{reason}`")]
    CustomPanic {
        /// The reason for the panic.
        reason: String,
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
    #[error("missing function with hash `{hash}`")]
    MissingFunction {
        /// Hash of function to look up.
        hash: Hash,
    },
    /// Failure to lookup instance function.
    #[error("missing instance function for instance `{instance}` with hash `{hash}`")]
    MissingInstanceFunction {
        /// The instance type we tried to look up function on.
        instance: ValueTypeInfo,
        /// Hash of function to look up.
        hash: Hash,
    },
    /// Instruction pointer went out-of-bounds.
    #[error("instruction pointer is out-of-bounds")]
    IpOutOfBounds,
    /// Tried to await something on the stack which can't be await:ed.
    #[error("unsupported target for .await")]
    UnsupportedAwait,
    /// A bad argument that was received to a function.
    #[error("bad argument `{argument}`")]
    BadArgument {
        /// The argument type.
        argument: ValueTypeInfo,
    },
    /// Unsupported binary operation.
    #[error("unsupported vm operation `{lhs} {op} {rhs}`")]
    UnsupportedBinaryOperation {
        /// Operation.
        op: &'static str,
        /// Left-hand side operator.
        lhs: ValueTypeInfo,
        /// Right-hand side operator.
        rhs: ValueTypeInfo,
    },
    /// Unsupported unary operation.
    #[error("unsupported vm operation `{op}{operand}`")]
    UnsupportedUnaryOperation {
        /// Operation.
        op: &'static str,
        /// Operand.
        operand: ValueTypeInfo,
    },
    /// Protocol not implemented on type.
    #[error("`{actual}` does not implement the `{protocol}` protocol")]
    MissingProtocol {
        /// The missing protocol.
        protocol: Protocol,
        /// The encountered argument.
        actual: ValueTypeInfo,
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
    /// Indicates a failure to convert from one type to another.
    #[error("failed to convert stack value to `{to}`: {error}")]
    StackConversionError {
        /// The source of the error.
        #[source]
        error: ValueError,
        /// The expected type to convert towards.
        to: &'static str,
    },
    /// Failure to convert from one type to another.
    #[error("failed to convert argument #{arg} to `{to}`: {error}")]
    ArgumentConversionError {
        /// The underlying stack error.
        #[source]
        error: ValueError,
        /// The argument location that was converted.
        arg: usize,
        /// The native type we attempt to convert to.
        to: &'static str,
    },
    /// Wrong number of arguments provided in call.
    #[error("wrong number of arguments `{actual}`, expected `{expected}`")]
    ArgumentCountMismatch {
        /// The actual number of arguments.
        actual: usize,
        /// The expected number of arguments.
        expected: usize,
    },
    /// Failure to convert return value.
    #[error("failed to convert return value `{ret}`")]
    ReturnConversionError {
        /// Error describing the failed conversion.
        #[source]
        error: ValueError,
        /// Type of the return value we attempted to convert.
        ret: &'static str,
    },
    /// An index set operation that is not supported.
    #[error("the index set operation `{target}[{index}] = {value}` is not supported")]
    UnsupportedIndexSet {
        /// The target type to set.
        target: ValueTypeInfo,
        /// The index to set.
        index: ValueTypeInfo,
        /// The value to set.
        value: ValueTypeInfo,
    },
    /// An index get operation that is not supported.
    #[error("the index get operation `{target}[{index}]` is not supported")]
    UnsupportedIndexGet {
        /// The target type to get.
        target: ValueTypeInfo,
        /// The index to get.
        index: ValueTypeInfo,
    },
    /// A vector index get operation that is not supported.
    #[error("the vector index get operation is not supported on `{target}`")]
    UnsupportedVecIndexGet {
        /// The target type we tried to perform the vector indexing on.
        target: ValueTypeInfo,
    },
    /// An tuple index get operation that is not supported.
    #[error("the tuple index get operation is not supported on `{target}`")]
    UnsupportedTupleIndexGet {
        /// The target type we tried to perform the tuple indexing on.
        target: ValueTypeInfo,
    },
    /// An tuple index set operation that is not supported.
    #[error("the tuple index set operation is not supported on `{target}`")]
    UnsupportedTupleIndexSet {
        /// The target type we tried to perform the tuple indexing on.
        target: ValueTypeInfo,
    },
    /// An object slot index get operation that is not supported.
    #[error("the object slot index get operation on `{target}` is not supported")]
    UnsupportedObjectSlotIndexGet {
        /// The target type we tried to perform the object indexing on.
        target: ValueTypeInfo,
    },
    /// An is operation is not supported.
    #[error("`{value} is {test_type}` is not supported")]
    UnsupportedIs {
        /// The argument that is not supported.
        value: ValueTypeInfo,
        /// The type that is not supported.
        test_type: ValueTypeInfo,
    },
    /// Encountered a value that could not be dereferenced.
    #[error("replace deref `*{target} = {value}` is not supported")]
    UnsupportedReplaceDeref {
        /// The type we try to assign to.
        target: ValueTypeInfo,
        /// The type we try to assign.
        value: ValueTypeInfo,
    },
    /// Encountered a value that could not be dereferenced.
    #[error("`*{actual_type}` is not supported")]
    UnsupportedDeref {
        /// The type that could not be de-referenced.
        actual_type: ValueTypeInfo,
    },
    /// Missing type.
    #[error("no type matching hash `{hash}`")]
    MissingType {
        /// Hash of the type missing.
        hash: Hash,
    },
    /// Encountered a value that could not be called as a function
    #[error("`{actual_type}` cannot be called since it's not a function")]
    UnsupportedCallFn {
        /// The type that could not be called.
        actual_type: ValueTypeInfo,
    },
    /// Tried to fetch an index in a vector that doesn't exist.
    #[error("missing index `{index}` in vector")]
    VecIndexMissing {
        /// The missing index.
        index: usize,
    },
    /// Tried to fetch an index in a tuple that doesn't exist.
    #[error("missing index `{index}` in tuple")]
    TupleIndexMissing {
        /// The missing index.
        index: usize,
    },
    /// Tried to fetch an index in an object that doesn't exist.
    #[error("missing index by static string slot `{slot}` in object")]
    ObjectIndexMissing {
        /// The static string slot corresponding to the index that is missing.
        slot: usize,
    },
    /// Error raised when we expect a specific external type but got another.
    #[error("expected slot `{expected}`, but found `{actual}`")]
    UnexpectedValueType {
        /// The type that was expected.
        expected: &'static str,
        /// The type that was found.
        actual: &'static str,
    },
    /// Error raised when we expected a vector of the given length.
    #[error("expected a vector of length `{expected}`, but found one with length `{actual}`")]
    ExpectedVecLength {
        /// The actual length observed.
        actual: usize,
        /// The expected vector length.
        expected: usize,
    },
    /// Tried to access an index that was missing on a type.
    #[error("missing index `{}` on `{target}`")]
    MissingIndex {
        /// Type where field did not exist.
        target: ValueTypeInfo,
        /// Index that we tried to access.
        index: Integer,
    },
    /// Missing a struct field.
    #[error("missing field `{field}` on `{target}`")]
    MissingField {
        /// Type where field did not exist.
        target: ValueTypeInfo,
        /// Field that was missing.
        field: String,
    },
    /// Error raised when we try to unwrap something that is not an option or
    /// result.
    #[error("expected result or option with value to unwrap, but got `{actual}`")]
    UnsupportedUnwrap {
        /// The actual operand.
        actual: ValueTypeInfo,
    },
    /// Error raised when we try to unwrap an Option that is not Some.
    #[error("expected Some value, but got `None`")]
    UnsupportedUnwrapNone,
    /// Error raised when we try to unwrap a Result that is not Ok.
    #[error("expected Ok value, but got `Err({err})`")]
    UnsupportedUnwrapErr {
        /// The error variant.
        err: ValueTypeInfo,
    },
    /// Value is not supported for `is-value` test.
    #[error("expected result or option as value, but got `{actual}`")]
    UnsupportedIsValueOperand {
        /// The actual operand.
        actual: ValueTypeInfo,
    },
    /// Trying to resume a generator that has completed.
    #[error("cannot resume generator that has completed")]
    GeneratorComplete,
}

impl VmError {
    /// Generate a custom panic.
    pub fn custom_panic<D>(reason: D) -> Self
    where
        D: fmt::Display,
    {
        Self::CustomPanic {
            reason: reason.to_string(),
        }
    }

    /// Convert into an unwinded vm error.
    pub fn into_unwinded(self, ip: usize) -> Self {
        match self {
            Self::UnwindedVmError { error, ip } => Self::UnwindedVmError { error, ip },
            error => Self::UnwindedVmError {
                error: Box::new(error),
                ip,
            },
        }
    }

    /// Unpack an unwinded error, if it is present.
    pub fn from_unwinded(self) -> (Self, Option<usize>) {
        match self {
            Self::UnwindedVmError { error, ip } => (*error, Some(ip)),
            error => (error, None),
        }
    }

    /// Unpack an unwinded error ref, if it is present.
    pub fn from_unwinded_ref(&self) -> (&Self, Option<usize>) {
        match self {
            Self::UnwindedVmError { error, ip } => (&*error, Some(*ip)),
            error => (error, None),
        }
    }
}

impl From<ValueError> for VmError {
    fn from(error: ValueError) -> Self {
        match error {
            ValueError::VmError { error } => *error,
            error => VmError::ValueError { error },
        }
    }
}

/// A call frame.
///
/// This is used to store the return point after an instruction has been run.
#[derive(Debug, Clone, Copy)]
struct CallFrame {
    /// The stored instruction pointer.
    ip: usize,
    /// The top of the stack at the time of the call to ensure stack isolation
    /// across function calls.
    ///
    /// I.e. a function should not be able to manipulate the size of any other
    /// stack than its own.
    stack_top: usize,
}

/// A stack which references variables indirectly from a slab.
#[derive(Debug, Clone)]
pub struct Vm {
    /// Context associated with virtual machine.
    context: Rc<Context>,
    /// Unit associated with virtual machine.
    unit: Rc<Unit>,
    /// The current instruction pointer.
    ip: usize,
    /// The current stack.
    stack: Stack,
    /// Frames relative to the stack.
    call_frames: Vec<CallFrame>,
}

impl Vm {
    /// Construct a new runestick virtual machine.
    pub const fn new(context: Rc<Context>, unit: Rc<Unit>) -> Self {
        Self::new_with_stack(context, unit, Stack::new())
    }

    /// Construct a new runestick virtual machine.
    pub const fn new_with_stack(context: Rc<Context>, unit: Rc<Unit>, stack: Stack) -> Self {
        Self {
            context,
            unit,
            ip: 0,
            stack,
            call_frames: Vec::new(),
        }
    }

    /// Test if the virtual machine is the same context and unit as specified.
    pub fn is_same(&self, context: &Rc<Context>, unit: &Rc<Unit>) -> bool {
        Rc::ptr_eq(&self.context, context) && Rc::ptr_eq(&self.unit, unit)
    }

    /// Set  the current instruction pointer.
    #[inline]
    pub fn set_ip(&mut self, ip: usize) {
        self.ip = ip;
    }

    /// Get the stack mutably.
    pub fn stack_mut(&mut self) -> &mut Stack {
        &mut self.stack
    }

    /// Access the context related to the virtual machine.
    pub fn context(&self) -> &Rc<Context> {
        &self.context
    }

    /// Access the underlying unit of the virtual machine.
    pub fn unit(&self) -> &Rc<Unit> {
        &self.unit
    }

    /// Reset this virtual machine, freeing all memory used.
    pub fn clear(&mut self) {
        self.ip = 0;
        self.stack.clear();
        self.call_frames.clear();
    }

    /// Access the current instruction pointer.
    pub fn ip(&self) -> usize {
        self.ip
    }

    /// Modify the current instruction pointer.
    pub fn modify_ip(&mut self, offset: isize) -> Result<(), VmError> {
        self.ip = if offset < 0 {
            self.ip.overflowing_sub(-offset as usize).0
        } else {
            self.ip.overflowing_add(offset as usize).0
        };

        Ok(())
    }

    /// Iterate over the stack, producing the value associated with each stack
    /// item.
    pub fn iter_stack_debug(&self) -> impl Iterator<Item = &Value> + '_ {
        self.stack.iter()
    }

    /// Call the given function in the given compilation unit.
    pub fn call_function<A, T, N>(&mut self, hash: N, args: A) -> Result<Task<'_, T>, VmError>
    where
        N: IntoTypeHash,
        A: IntoArgs,
        T: FromValue,
    {
        let hash = hash.into_type_hash();

        let function = self
            .unit
            .lookup(hash)
            .ok_or_else(|| VmError::MissingFunction { hash })?;

        if function.signature.args != A::count() {
            return Err(VmError::ArgumentCountMismatch {
                actual: A::count(),
                expected: function.signature.args,
            });
        }

        let offset = match function.kind {
            // NB: we ignore the calling convention.
            // everything is just async when called externally.
            UnitFnKind::Offset { offset, .. } => offset,
            _ => {
                return Err(VmError::MissingFunction { hash });
            }
        };

        self.ip = offset;
        self.stack.clear();

        // Safety: we bind the lifetime of the arguments to the outgoing task,
        // ensuring that the task won't outlive any references passed in.
        args.into_args(&mut self.stack)?;

        Ok(Task {
            vm: self,
            _marker: marker::PhantomData,
        })
    }

    /// Run the given program on the virtual machine.
    pub fn run<T>(&mut self) -> Task<'_, T>
    where
        T: FromValue,
    {
        Task {
            vm: self,
            _marker: marker::PhantomData,
        }
    }

    async fn op_await(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        match value {
            Value::Future(future) => {
                let value = future.borrow_mut()?.await?;
                self.stack.push(value);
                Ok(())
            }
            _ => Err(VmError::UnsupportedAwait),
        }
    }

    async fn op_select(&mut self, len: usize) -> Result<(), VmError> {
        use futures::stream::StreamExt as _;

        let mut futures = futures::stream::FuturesUnordered::new();

        for branch in 0..len {
            let future = self.stack.pop()?.into_future()?.owned_mut()?;

            if !future.is_completed() {
                futures.push(SelectFuture::new(branch, future));
            }
        }

        // NB: nothing to poll.
        if futures.is_empty() {
            return Ok(());
        }

        let (branch, value) = futures.next().await.unwrap()?;
        let branch = ToValue::to_value(branch)?;

        self.stack.push(value);
        self.stack.push(branch);
        Ok(())
    }

    /// Helper function to call an instance function.
    fn call_instance_fn<H, A>(&mut self, target: &Value, hash: H, args: A) -> Result<bool, VmError>
    where
        H: IntoTypeHash,
        A: IntoArgs,
    {
        let count = A::count() + 1;
        let hash = Hash::instance_function(target.value_type()?, hash.into_type_hash());

        if let Some(info) = self.unit.lookup(hash) {
            if info.signature.args != count {
                return Err(VmError::ArgumentCountMismatch {
                    actual: count,
                    expected: info.signature.args,
                });
            }

            if let UnitFnKind::Offset { offset, call } = &info.kind {
                let offset = *offset;
                let call = *call;

                args.into_args(&mut self.stack)?;

                self.stack.push(target.clone());
                self.call_offset_fn(offset, call, count)?;
                return Ok(true);
            }
        }

        let handler = match self.context.lookup(hash) {
            Some(handler) => handler,
            None => return Ok(false),
        };

        args.into_args(&mut self.stack)?;

        self.stack.push(target.clone());
        handler(&mut self.stack, count)?;
        Ok(true)
    }

    /// Pop a number of values from the stack.
    fn op_popn(&mut self, n: usize) -> Result<(), VmError> {
        self.stack.popn(n)?;
        Ok(())
    }

    /// pop-and-jump-if instruction.
    fn op_pop_and_jump_if(&mut self, count: usize, offset: isize) -> Result<(), VmError> {
        if !self.stack.pop()?.into_bool()? {
            return Ok(());
        }

        self.stack.popn(count)?;
        self.modify_ip(offset)?;
        Ok(())
    }

    /// pop-and-jump-if-not instruction.
    fn op_pop_and_jump_if_not(&mut self, count: usize, offset: isize) -> Result<(), VmError> {
        if self.stack.pop()?.into_bool()? {
            return Ok(());
        }

        self.stack.popn(count)?;
        self.modify_ip(offset)?;
        Ok(())
    }

    /// Pop a number of values from the stack, while preserving the top of the
    /// stack.
    fn op_clean(&mut self, n: usize) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        self.op_popn(n)?;
        self.stack.push(value);
        Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    fn op_copy(&mut self, offset: usize) -> Result<(), VmError> {
        let value = self.stack.at_offset(offset)?.clone();
        self.stack.push(value);
        Ok(())
    }

    #[inline]
    fn op_drop(&mut self, offset: usize) -> Result<(), VmError> {
        let _ = self.stack.at_offset(offset)?;
        Ok(())
    }

    /// Duplicate the value at the top of the stack.
    fn op_dup(&mut self) -> Result<(), VmError> {
        let value = self.stack.last()?.clone();
        self.stack.push(value);
        Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    fn op_replace(&mut self, offset: usize) -> Result<(), VmError> {
        let mut value = self.stack.pop()?;
        let stack_value = self.stack.at_offset_mut(offset)?;
        mem::swap(stack_value, &mut value);
        Ok(())
    }

    fn internal_boolean_ops(
        &mut self,
        int_op: impl FnOnce(i64, i64) -> bool,
        float_op: impl FnOnce(f64, f64) -> bool,
        op: &'static str,
    ) -> Result<(), VmError> {
        let rhs = self.stack.pop()?;
        let lhs = self.stack.pop()?;

        let out = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => int_op(lhs, rhs),
            (Value::Float(lhs), Value::Float(rhs)) => float_op(lhs, rhs),
            (lhs, rhs) => {
                return Err(VmError::UnsupportedBinaryOperation {
                    op,
                    lhs: lhs.type_info()?,
                    rhs: rhs.type_info()?,
                })
            }
        };

        self.stack.push(Value::Bool(out));
        Ok(())
    }

    fn op_gt(&mut self) -> Result<(), VmError> {
        self.internal_boolean_ops(|a, b| a > b, |a, b| a > b, ">")?;
        Ok(())
    }

    fn op_gte(&mut self) -> Result<(), VmError> {
        self.internal_boolean_ops(|a, b| a >= b, |a, b| a >= b, ">=")?;
        Ok(())
    }

    fn op_lt(&mut self) -> Result<(), VmError> {
        self.internal_boolean_ops(|a, b| a < b, |a, b| a < b, "<")?;
        Ok(())
    }

    fn op_lte(&mut self) -> Result<(), VmError> {
        self.internal_boolean_ops(|a, b| a <= b, |a, b| a <= b, "<=")?;
        Ok(())
    }

    /// Push a new call frame.
    ///
    /// This will cause the `args` number of elements on the stack to be
    /// associated and accessible to the new call frame.
    pub fn push_call_frame(&mut self, ip: usize, args: usize) -> Result<(), VmError> {
        let stack_top = self.stack.push_stack_top(args)?;

        self.call_frames.push(CallFrame {
            ip: self.ip,
            stack_top,
        });

        self.ip = ip.overflowing_sub(1).0;
        Ok(())
    }

    /// Construct a tuple.
    #[inline]
    fn allocate_typed_tuple(&mut self, hash: Hash, args: usize) -> Result<Value, VmError> {
        let mut tuple = Vec::new();

        for _ in 0..args {
            tuple.push(self.stack.pop()?);
        }

        let typed_tuple = Shared::new(TypedTuple {
            hash,
            tuple: tuple.into_boxed_slice(),
        });

        Ok(Value::TypedTuple(typed_tuple))
    }

    /// Construct a tuple variant.
    #[inline]
    fn allocate_tuple_variant(
        &mut self,
        enum_hash: Hash,
        hash: Hash,
        args: usize,
    ) -> Result<Value, VmError> {
        let mut tuple = Vec::new();

        for _ in 0..args {
            tuple.push(self.stack.pop()?);
        }

        let typed_tuple = Shared::new(VariantTuple {
            enum_hash,
            hash,
            tuple: tuple.into_boxed_slice(),
        });

        Ok(Value::VariantTuple(typed_tuple))
    }

    /// Pop a call frame and return it.
    fn pop_call_frame(&mut self) -> Result<bool, VmError> {
        let frame = match self.call_frames.pop() {
            Some(frame) => frame,
            None => {
                self.stack.check_stack_top()?;
                return Ok(true);
            }
        };

        self.stack.pop_stack_top(frame.stack_top)?;
        self.ip = frame.ip;
        Ok(false)
    }

    /// Pop the last value on the stack and evaluate it as `T`.
    fn pop_decode<T>(&mut self) -> Result<T, VmError>
    where
        T: FromValue,
    {
        let value = self.stack.pop()?;

        let value = match T::from_value(value) {
            Ok(value) => value,
            Err(error) => {
                return Err(VmError::StackConversionError {
                    error,
                    to: any::type_name::<T>(),
                });
            }
        };

        Ok(value)
    }

    /// Optimized function to test if two value pointers are deeply equal to
    /// each other.
    ///
    /// This is the basis for the eq operation (`==`).
    fn value_ptr_eq(&self, a: &Value, b: &Value) -> Result<bool, VmError> {
        Ok(match (a, b) {
            (Value::Unit, Value::Unit) => true,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Vec(a), Value::Vec(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                if a.len() != b.len() {
                    return Ok(false);
                }

                for (a, b) in a.iter().zip(b.iter()) {
                    if !self.value_ptr_eq(a, b)? {
                        return Ok(false);
                    }
                }

                true
            }
            (Value::Object(a), Value::Object(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                if a.len() != b.len() {
                    return Ok(false);
                }

                for (key, a) in a.iter() {
                    let b = match b.get(key) {
                        Some(b) => b,
                        None => return Ok(false),
                    };

                    if !self.value_ptr_eq(a, b)? {
                        return Ok(false);
                    }
                }

                true
            }
            (Value::String(a), Value::String(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;
                *a == *b
            }
            (Value::StaticString(a), Value::String(b)) => {
                let b = b.borrow_ref()?;
                ***a == *b
            }
            (Value::String(a), Value::StaticString(b)) => {
                let a = a.borrow_ref()?;
                *a == ***b
            }
            // fast string comparison: exact string slot.
            (Value::StaticString(a), Value::StaticString(b)) => **a == **b,
            // fast external comparison by slot.
            // TODO: implement ptr equals.
            // (Value::Any(a), Value::Any(b)) => a == b,
            _ => false,
        })
    }

    /// Optimized equality implementation.
    #[inline]
    fn op_eq(&mut self) -> Result<(), VmError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        self.stack.push(Value::Bool(self.value_ptr_eq(&a, &b)?));
        Ok(())
    }

    /// Optimized inequality implementation.
    #[inline]
    fn op_neq(&mut self) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        self.stack.push(Value::Bool(!self.value_ptr_eq(&a, &b)?));
        Ok(())
    }

    /// Perform a jump operation.
    #[inline]
    fn op_jump(&mut self, offset: isize) -> Result<(), VmError> {
        self.modify_ip(offset)?;
        Ok(())
    }

    /// Perform a conditional jump operation.
    #[inline]
    fn op_jump_if(&mut self, offset: isize) -> Result<(), VmError> {
        if self.stack.pop()?.into_bool()? {
            self.modify_ip(offset)?;
        }

        Ok(())
    }

    /// Perform a conditional jump operation.
    #[inline]
    fn op_jump_if_not(&mut self, offset: isize) -> Result<(), VmError> {
        if !self.stack.pop()?.into_bool()? {
            self.modify_ip(offset)?;
        }

        Ok(())
    }

    /// Perform a branch-conditional jump operation.
    #[inline]
    fn op_jump_if_branch(&mut self, branch: i64, offset: isize) -> Result<(), VmError> {
        if let Some(Value::Integer(current)) = self.stack.peek() {
            if *current == branch {
                self.modify_ip(offset)?;
                self.stack.pop()?;
            }
        }

        Ok(())
    }

    /// Construct a new vec.
    #[inline]
    fn op_vec(&mut self, count: usize) -> Result<(), VmError> {
        let mut vec = Vec::with_capacity(count);

        for _ in 0..count {
            vec.push(self.stack.pop()?);
        }

        let value = Value::Vec(Shared::new(vec));
        self.stack.push(value);
        Ok(())
    }

    /// Construct a new tuple.
    #[inline]
    fn op_tuple(&mut self, count: usize) -> Result<(), VmError> {
        let mut tuple = Vec::with_capacity(count);

        for _ in 0..count {
            tuple.push(self.stack.pop()?);
        }

        let value = Value::Tuple(Shared::new(Tuple::from(tuple)));
        self.stack.push(value);
        Ok(())
    }

    #[inline]
    fn op_not(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let value = match value {
            Value::Bool(value) => Value::Bool(!value),
            other => {
                let operand = other.type_info()?;
                return Err(VmError::UnsupportedUnaryOperation { op: "!", operand });
            }
        };

        self.stack.push(value);
        Ok(())
    }

    /// Internal impl of a numeric operation.
    fn internal_numeric_op<H, E, I, F>(
        &mut self,
        hash: H,
        error: E,
        integer_op: I,
        float_op: F,
        op: &'static str,
    ) -> Result<(), VmError>
    where
        H: IntoTypeHash,
        E: Copy + FnOnce() -> VmError,
        I: FnOnce(i64, i64) -> Option<i64>,
        F: FnOnce(f64, f64) -> f64,
    {
        let rhs = self.stack.pop()?;
        let lhs = self.stack.pop()?;

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                let out = integer_op(lhs, rhs).ok_or_else(error)?;
                self.stack.push(Value::Integer(out));
                return Ok(());
            }
            (Value::Float(lhs), Value::Float(rhs)) => {
                let out = float_op(lhs, rhs);
                self.stack.push(Value::Float(out));
                return Ok(());
            }
            (lhs, rhs) => (lhs.clone(), rhs),
        };

        if !self.call_instance_fn(&lhs, hash, (&rhs,))? {
            return Err(VmError::UnsupportedBinaryOperation {
                op,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            });
        }

        Ok(())
    }

    #[inline]
    fn op_add(&mut self) -> Result<(), VmError> {
        self.internal_numeric_op(
            crate::ADD,
            || VmError::Overflow,
            i64::checked_add,
            std::ops::Add::add,
            "+",
        )?;
        Ok(())
    }

    #[inline]
    fn op_sub(&mut self) -> Result<(), VmError> {
        self.internal_numeric_op(
            crate::SUB,
            || VmError::Underflow,
            i64::checked_sub,
            std::ops::Sub::sub,
            "-",
        )?;
        Ok(())
    }

    #[inline]
    fn op_mul(&mut self) -> Result<(), VmError> {
        self.internal_numeric_op(
            crate::ADD,
            || VmError::Overflow,
            i64::checked_mul,
            std::ops::Mul::mul,
            "*",
        )?;
        Ok(())
    }

    #[inline]
    fn op_div(&mut self) -> Result<(), VmError> {
        self.internal_numeric_op(
            crate::ADD,
            || VmError::DivideByZero,
            i64::checked_div,
            std::ops::Div::div,
            "+",
        )?;
        Ok(())
    }

    #[inline]
    fn op_rem(&mut self) -> Result<(), VmError> {
        self.internal_numeric_op(
            crate::REM,
            || VmError::DivideByZero,
            i64::checked_rem,
            std::ops::Rem::rem,
            "%",
        )?;
        Ok(())
    }

    fn internal_op_assign<H, E, I, F>(
        &mut self,
        offset: usize,
        hash: H,
        error: E,
        integer_op: I,
        float_op: F,
        op: &'static str,
    ) -> Result<(), VmError>
    where
        H: IntoTypeHash,
        E: Copy + FnOnce() -> VmError,
        I: FnOnce(i64, i64) -> Option<i64>,
        F: FnOnce(f64, f64) -> f64,
    {
        let rhs = self.stack.pop()?;
        let lhs = self.stack.at_offset_mut(offset)?;

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                let out = integer_op(*lhs, rhs).ok_or_else(error)?;
                *lhs = out;
                return Ok(());
            }
            (Value::Float(lhs), Value::Float(rhs)) => {
                let out = float_op(*lhs, rhs);
                *lhs = out;
                return Ok(());
            }
            (lhs, rhs) => (lhs.clone(), rhs),
        };

        if !self.call_instance_fn(&lhs, hash, (&rhs,))? {
            return Err(VmError::UnsupportedBinaryOperation {
                op,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            });
        }

        self.stack.pop()?;
        Ok(())
    }

    #[inline]
    fn op_add_assign(&mut self, offset: usize) -> Result<(), VmError> {
        self.internal_op_assign(
            offset,
            crate::ADD_ASSIGN,
            || VmError::Overflow,
            i64::checked_add,
            std::ops::Add::add,
            "+=",
        )?;
        Ok(())
    }

    #[inline]
    fn op_sub_assign(&mut self, offset: usize) -> Result<(), VmError> {
        self.internal_op_assign(
            offset,
            crate::SUB_ASSIGN,
            || VmError::Underflow,
            i64::checked_sub,
            std::ops::Sub::sub,
            "-=",
        )?;
        Ok(())
    }

    #[inline]
    fn op_mul_assign(&mut self, offset: usize) -> Result<(), VmError> {
        self.internal_op_assign(
            offset,
            crate::MUL_ASSIGN,
            || VmError::Overflow,
            i64::checked_mul,
            std::ops::Mul::mul,
            "*=",
        )?;
        Ok(())
    }

    #[inline]
    fn op_div_assign(&mut self, offset: usize) -> Result<(), VmError> {
        self.internal_op_assign(
            offset,
            crate::DIV_ASSIGN,
            || VmError::DivideByZero,
            i64::checked_div,
            std::ops::Div::div,
            "/=",
        )?;
        Ok(())
    }

    /// Perform an index set operation.
    #[inline]
    fn op_index_set(&mut self) -> Result<(), VmError> {
        let target = self.stack.pop()?;
        let index = self.stack.pop()?;
        let value = self.stack.pop()?;

        // This is a useful pattern.
        #[allow(clippy::never_loop)]
        loop {
            // NB: local storage for string.
            let local_field;

            let field = match &index {
                Value::String(string) => {
                    local_field = string.borrow_ref()?;
                    local_field.as_str()
                }
                Value::StaticString(string) => string.as_ref(),
                _ => break,
            };

            match &target {
                Value::Object(object) => {
                    let mut object = object.borrow_mut()?;
                    object.insert(field.to_owned(), value);
                    return Ok(());
                }
                Value::TypedObject(typed_object) => {
                    let mut typed_object = typed_object.borrow_mut()?;

                    if let Some(v) = typed_object.object.get_mut(field) {
                        *v = value;
                        return Ok(());
                    }

                    return Err(VmError::MissingField {
                        field: field.to_owned(),
                        target: typed_object.type_info(),
                    });
                }
                Value::VariantObject(variant_object) => {
                    let mut variant_object = variant_object.borrow_mut()?;

                    if let Some(v) = variant_object.object.get_mut(field) {
                        *v = value;
                        return Ok(());
                    }

                    return Err(VmError::MissingField {
                        field: field.to_owned(),
                        target: variant_object.type_info(),
                    });
                }
                _ => break,
            }
        }

        if !self.call_instance_fn(&target, crate::INDEX_SET, (&index, &value))? {
            return Err(VmError::UnsupportedIndexSet {
                target: target.type_info()?,
                index: index.type_info()?,
                value: value.type_info()?,
            });
        }

        Ok(())
    }

    #[inline]
    fn op_return(&mut self) -> Result<bool, VmError> {
        let return_value = self.stack.pop()?;
        let exit = self.pop_call_frame()?;
        self.stack.push(return_value);
        Ok(exit)
    }

    #[inline]
    fn op_return_unit(&mut self) -> Result<bool, VmError> {
        let exit = self.pop_call_frame()?;
        self.stack.push(Value::Unit);
        Ok(exit)
    }

    #[inline]
    fn op_load_instance_fn(&mut self, hash: Hash) -> Result<(), VmError> {
        let instance = self.stack.pop()?;
        let ty = instance.value_type()?;
        let hash = Hash::instance_function(ty, hash);
        self.stack.push(Value::Type(hash));
        Ok(())
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_object_like_index_get(&mut self, target: &Value, field: &str) -> Result<bool, VmError> {
        let value = match &target {
            Value::Object(target) => target.borrow_ref()?.get(field).cloned(),
            Value::TypedObject(target) => target.borrow_ref()?.object.get(field).cloned(),
            Value::VariantObject(target) => target.borrow_ref()?.object.get(field).cloned(),
            _ => return Ok(false),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return Err(VmError::MissingField {
                    target: target.type_info()?,
                    field: field.to_owned(),
                });
            }
        };

        self.stack.push(value);
        Ok(true)
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_tuple_like_index_get(target: &Value, index: usize) -> Result<Option<Value>, VmError> {
        let value = match target {
            Value::Unit => None,
            Value::Tuple(tuple) => tuple.borrow_ref()?.get(index).cloned(),
            Value::Vec(vec) => vec.borrow_ref()?.get(index).cloned(),
            Value::Result(result) => {
                let result = result.borrow_ref()?;

                match &*result {
                    Ok(value) if index == 0 => Some(value.clone()),
                    Err(value) if index == 0 => Some(value.clone()),
                    _ => None,
                }
            }
            Value::Option(option) => {
                let option = option.borrow_ref()?;

                match &*option {
                    Some(value) if index == 0 => Some(value.clone()),
                    _ => None,
                }
            }
            Value::GeneratorState(state) => {
                use crate::GeneratorState::*;
                let state = state.borrow_ref()?;

                match &*state {
                    Yielded(value) if index == 0 => Some(value.clone()),
                    Complete(value) if index == 0 => Some(value.clone()),
                    _ => None,
                }
            }
            Value::TypedTuple(typed_tuple) => {
                let typed_tuple = typed_tuple.borrow_ref()?;
                typed_tuple.tuple.get(index).cloned()
            }
            Value::VariantTuple(variant_tuple) => {
                let variant_tuple = variant_tuple.borrow_ref()?;
                variant_tuple.tuple.get(index).cloned()
            }
            _ => return Ok(None),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return Err(VmError::MissingIndex {
                    target: target.type_info()?,
                    index: Integer::Usize(index),
                });
            }
        };

        Ok(Some(value))
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_tuple_like_index_set(
        target: &Value,
        index: usize,
        value: Value,
    ) -> Result<bool, VmError> {
        match target {
            Value::Unit => Ok(false),
            Value::Tuple(tuple) => {
                let mut tuple = tuple.borrow_mut()?;

                if let Some(target) = tuple.get_mut(index) {
                    *target = value;
                    return Ok(true);
                }

                Ok(false)
            }
            Value::Vec(vec) => {
                let mut vec = vec.borrow_mut()?;

                if let Some(target) = vec.get_mut(index) {
                    *target = value;
                    return Ok(true);
                }

                Ok(false)
            }
            Value::Result(result) => {
                let mut result = result.borrow_mut()?;

                let target = match &mut *result {
                    Ok(ok) if index == 0 => ok,
                    Err(err) if index == 0 => err,
                    _ => return Ok(false),
                };

                *target = value;
                Ok(true)
            }
            Value::Option(option) => {
                let mut option = option.borrow_mut()?;

                let target = match &mut *option {
                    Some(some) if index == 0 => some,
                    _ => return Ok(false),
                };

                *target = value;
                Ok(true)
            }
            Value::TypedTuple(typed_tuple) => {
                let mut typed_tuple = typed_tuple.borrow_mut()?;

                if let Some(target) = typed_tuple.tuple.get_mut(index) {
                    *target = value;
                    return Ok(true);
                }

                Ok(false)
            }
            Value::VariantTuple(variant_tuple) => {
                let mut variant_tuple = variant_tuple.borrow_mut()?;

                if let Some(target) = variant_tuple.tuple.get_mut(index) {
                    *target = value;
                    return Ok(true);
                }

                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Perform an index get operation.
    #[inline]
    fn op_index_get(&mut self) -> Result<(), VmError> {
        let target = self.stack.pop()?;
        let index = self.stack.pop()?;

        // This is a useful pattern.
        #[allow(clippy::never_loop)]
        loop {
            match &index {
                Value::String(string) => {
                    let string_ref = string.borrow_ref()?;

                    if self.try_object_like_index_get(&target, string_ref.as_str())? {
                        return Ok(());
                    }
                }
                Value::StaticString(string) => {
                    if self.try_object_like_index_get(&target, string.as_ref())? {
                        return Ok(());
                    }
                }
                Value::Integer(index) => {
                    use std::convert::TryInto as _;

                    let index = match (*index).try_into() {
                        Ok(index) => index,
                        Err(..) => {
                            return Err(VmError::MissingIndex {
                                target: target.type_info()?,
                                index: Integer::I64(*index),
                            });
                        }
                    };

                    if let Some(value) = Self::try_tuple_like_index_get(&target, index)? {
                        self.stack.push(value);
                        return Ok(());
                    }
                }
                _ => break,
            };
        }

        if !self.call_instance_fn(&target, crate::INDEX_GET, (&index,))? {
            return Err(VmError::UnsupportedIndexGet {
                target: target.type_info()?,
                index: index.type_info()?,
            });
        }

        Ok(())
    }

    /// Perform an index get operation specialized for tuples.
    #[inline]
    fn op_tuple_index_get(&mut self, index: usize) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        if let Some(value) = Self::try_tuple_like_index_get(&value, index)? {
            self.stack.push(value);
            return Ok(());
        }

        Err(VmError::UnsupportedTupleIndexGet {
            target: value.type_info()?,
        })
    }

    /// Perform an index get operation specialized for tuples.
    #[inline]
    fn op_tuple_index_set(&mut self, index: usize) -> Result<(), VmError> {
        let tuple = self.stack.pop()?;
        let value = self.stack.pop()?;

        if Self::try_tuple_like_index_set(&tuple, index, value)? {
            return Ok(());
        }

        Err(VmError::UnsupportedTupleIndexSet {
            target: tuple.type_info()?,
        })
    }

    /// Perform an index get operation specialized for tuples.
    #[inline]
    fn op_tuple_index_get_at(&mut self, offset: usize, index: usize) -> Result<(), VmError> {
        let value = self.stack.at_offset(offset)?;

        if let Some(value) = Self::try_tuple_like_index_get(value, index)? {
            self.stack.push(value);
            return Ok(());
        }

        Err(VmError::UnsupportedTupleIndexGet {
            target: value.type_info()?,
        })
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_object_slot_index_get(
        unit: &Unit,
        target: &Value,
        string_slot: usize,
    ) -> Result<Option<Value>, VmError> {
        Ok(match target {
            Value::Object(object) => {
                let index = unit.lookup_string(string_slot)?;
                let object = object.borrow_ref()?;

                match object.get(&***index).cloned() {
                    Some(value) => Some(value),
                    None => {
                        return Err(VmError::ObjectIndexMissing { slot: string_slot });
                    }
                }
            }
            Value::TypedObject(typed_object) => {
                let index = unit.lookup_string(string_slot)?;
                let typed_object = typed_object.borrow_ref()?;

                match typed_object.object.get(&***index).cloned() {
                    Some(value) => Some(value),
                    None => {
                        return Err(VmError::ObjectIndexMissing { slot: string_slot });
                    }
                }
            }
            Value::VariantObject(variant_object) => {
                let index = unit.lookup_string(string_slot)?;
                let variant_object = variant_object.borrow_ref()?;

                match variant_object.object.get(&***index).cloned() {
                    Some(value) => Some(value),
                    None => {
                        return Err(VmError::ObjectIndexMissing { slot: string_slot });
                    }
                }
            }
            _ => None,
        })
    }

    /// Perform a specialized index get operation on an object.
    #[inline]
    fn op_object_slot_index_get(&mut self, string_slot: usize) -> Result<(), VmError> {
        let target = self.stack.pop()?;

        if let Some(value) = Self::try_object_slot_index_get(&self.unit, &target, string_slot)? {
            self.stack.push(value);
            return Ok(());
        }

        let target = target.type_info()?;
        Err(VmError::UnsupportedObjectSlotIndexGet { target })
    }

    /// Perform a specialized index get operation on an object.
    #[inline]
    fn op_object_slot_index_get_at(
        &mut self,
        offset: usize,
        string_slot: usize,
    ) -> Result<(), VmError> {
        let target = self.stack.at_offset(offset)?;

        if let Some(value) = Self::try_object_slot_index_get(&self.unit, target, string_slot)? {
            self.stack.push(value);
            return Ok(());
        }

        let target = target.type_info()?;
        Err(VmError::UnsupportedObjectSlotIndexGet { target })
    }

    /// Operation to allocate an object.
    #[inline]
    fn op_object(&mut self, slot: usize) -> Result<(), VmError> {
        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or_else(|| VmError::MissingStaticObjectKeys { slot })?;

        let mut object = Object::with_capacity(keys.len());

        for key in keys {
            let value = self.stack.pop()?;
            object.insert(key.clone(), value);
        }

        let value = Value::Object(Shared::new(object));
        self.stack.push(value);
        Ok(())
    }

    /// Operation to allocate an object.
    #[inline]
    fn op_typed_object(&mut self, hash: Hash, slot: usize) -> Result<(), VmError> {
        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or_else(|| VmError::MissingStaticObjectKeys { slot })?;

        let mut object = Object::with_capacity(keys.len());

        for key in keys {
            let value = self.stack.pop()?;
            object.insert(key.clone(), value);
        }

        let object = TypedObject { hash, object };
        let value = Value::TypedObject(Shared::new(object));
        self.stack.push(value);
        Ok(())
    }

    /// Operation to allocate an object.
    #[inline]
    fn op_variant_object(
        &mut self,
        enum_hash: Hash,
        hash: Hash,
        slot: usize,
    ) -> Result<(), VmError> {
        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or_else(|| VmError::MissingStaticObjectKeys { slot })?;

        let mut object = Object::with_capacity(keys.len());

        for key in keys {
            let value = self.stack.pop()?;
            object.insert(key.clone(), value);
        }

        let object = VariantObject {
            enum_hash,
            hash,
            object,
        };
        let value = Value::VariantObject(Shared::new(object));
        self.stack.push(value);
        Ok(())
    }

    #[inline]
    fn op_string(&mut self, slot: usize) -> Result<(), VmError> {
        let string = self.unit.lookup_string(slot)?;
        let value = Value::StaticString(StaticString::from(string.clone()));
        self.stack.push(value);
        Ok(())
    }

    #[inline]
    fn op_bytes(&mut self, slot: usize) -> Result<(), VmError> {
        let bytes = self.unit.lookup_bytes(slot)?.to_owned();
        let value = Value::Bytes(Shared::new(Bytes::from_vec(bytes)));
        self.stack.push(value);
        Ok(())
    }

    /// Optimize operation to perform string concatenation.
    #[inline]
    fn op_string_concat(&mut self, len: usize, size_hint: usize) -> Result<(), VmError> {
        let mut buf = String::with_capacity(size_hint);

        for _ in 0..len {
            let value = self.stack.pop()?;

            match value {
                Value::String(string) => {
                    buf.push_str(&*string.borrow_ref()?);
                }
                Value::StaticString(string) => {
                    buf.push_str(string.as_ref());
                }
                Value::Integer(integer) => {
                    let mut buffer = itoa::Buffer::new();
                    buf.push_str(buffer.format(integer));
                }
                Value::Float(float) => {
                    let mut buffer = ryu::Buffer::new();
                    buf.push_str(buffer.format(float));
                }
                actual => {
                    let b = Shared::new(std::mem::take(&mut buf));

                    if !self.call_instance_fn(
                        &actual,
                        crate::STRING_DISPLAY,
                        (Value::String(b.clone()),),
                    )? {
                        return Err(VmError::MissingProtocol {
                            protocol: crate::STRING_DISPLAY,
                            actual: actual.type_info()?,
                        });
                    }

                    let value = self.pop_decode::<fmt::Result>()?;

                    if let Err(fmt::Error) = value {
                        return Err(VmError::FormatError);
                    }

                    buf = b.take()?;
                }
            }
        }

        self.stack.push(Value::String(Shared::new(buf)));
        Ok(())
    }

    #[inline]
    fn op_unwrap(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let value = match value {
            Value::Option(option) => match option.take()? {
                Some(value) => value,
                None => {
                    return Err(VmError::UnsupportedUnwrapNone);
                }
            },
            Value::Result(result) => match result.take()? {
                Ok(value) => value,
                Err(err) => {
                    return Err(VmError::UnsupportedUnwrapErr {
                        err: err.type_info()?,
                    });
                }
            },
            other => {
                return Err(VmError::UnsupportedUnwrap {
                    actual: other.type_info()?,
                });
            }
        };

        self.stack.push(value);
        Ok(())
    }

    /// Internal implementation of the instance check.
    fn is_instance(&mut self) -> Result<bool, VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;

        let hash = match b {
            Value::Type(hash) => hash,
            _ => {
                return Err(VmError::UnsupportedIs {
                    value: a.type_info()?,
                    test_type: b.type_info()?,
                });
            }
        };

        Ok(a.value_type()? == hash)
    }

    #[inline]
    fn op_is(&mut self) -> Result<(), VmError> {
        let is_instance = self.is_instance()?;
        self.stack.push(Value::Bool(is_instance));
        Ok(())
    }

    #[inline]
    fn op_is_not(&mut self) -> Result<(), VmError> {
        let is_instance = self.is_instance()?;
        self.stack.push(Value::Bool(!is_instance));
        Ok(())
    }

    #[inline]
    fn op_is_unit(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        self.stack.push(Value::Bool(matches!(value, Value::Unit)));
        Ok(())
    }

    /// Test if the top of the stack is an error.
    #[inline]
    fn op_is_value(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let is_value = match value {
            Value::Result(result) => result.borrow_ref()?.is_ok(),
            Value::Option(option) => option.borrow_ref()?.is_some(),
            other => {
                return Err(VmError::UnsupportedIsValueOperand {
                    actual: other.type_info()?,
                })
            }
        };

        self.stack.push(Value::Bool(is_value));
        Ok(())
    }

    fn internal_boolean_op(
        &mut self,
        bool_op: impl FnOnce(bool, bool) -> bool,
        op: &'static str,
    ) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;

        let out = match (a, b) {
            (Value::Bool(a), Value::Bool(b)) => bool_op(a, b),
            (lhs, rhs) => {
                return Err(VmError::UnsupportedBinaryOperation {
                    op,
                    lhs: lhs.type_info()?,
                    rhs: rhs.type_info()?,
                });
            }
        };

        self.stack.push(Value::Bool(out));
        Ok(())
    }

    /// Operation associated with `and` instruction.
    #[inline]
    fn op_and(&mut self) -> Result<(), VmError> {
        self.internal_boolean_op(|a, b| a && b, "&&")?;
        Ok(())
    }

    /// Operation associated with `or` instruction.
    #[inline]
    fn op_or(&mut self) -> Result<(), VmError> {
        self.internal_boolean_op(|a, b| a || b, "||")?;
        Ok(())
    }

    #[inline]
    fn op_eq_byte(&mut self, byte: u8) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.stack.push(Value::Bool(match value {
            Value::Byte(actual) => actual == byte,
            _ => false,
        }));

        Ok(())
    }

    #[inline]
    fn op_eq_character(&mut self, character: char) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.stack.push(Value::Bool(match value {
            Value::Char(actual) => actual == character,
            _ => false,
        }));

        Ok(())
    }

    #[inline]
    fn op_eq_integer(&mut self, integer: i64) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.stack.push(Value::Bool(match value {
            Value::Integer(actual) => actual == integer,
            _ => false,
        }));

        Ok(())
    }

    /// Test if the top of stack is equal to the string at the given static
    /// string location.
    #[inline]
    fn op_eq_static_string(&mut self, slot: usize) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let equal = match value {
            Value::String(actual) => {
                let string = self.unit.lookup_string(slot)?;
                let actual = actual.borrow_ref()?;
                *actual == ***string
            }
            Value::StaticString(actual) => {
                let string = self.unit.lookup_string(slot)?;
                **actual == ***string
            }
            _ => false,
        };

        self.stack.push(Value::Bool(equal));

        Ok(())
    }

    #[inline]
    fn op_match_sequence(&mut self, ty: TypeCheck, len: usize, exact: bool) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let result = self.on_tuple(ty, &value, move |tuple| {
            if exact {
                tuple.len() == len
            } else {
                tuple.len() >= len
            }
        })?;

        self.stack.push(Value::Bool(result.unwrap_or_default()));
        Ok(())
    }

    #[inline]
    fn op_match_object(
        &mut self,
        type_check: TypeCheck,
        slot: usize,
        exact: bool,
    ) -> Result<(), VmError> {
        let result = self.on_object_keys(type_check, slot, |object, keys| {
            if exact {
                if object.len() != keys.len() {
                    return false;
                }
            } else if object.len() < keys.len() {
                return false;
            }

            let mut is_match = true;

            for key in keys {
                if !object.contains_key(key) {
                    is_match = false;
                    break;
                }
            }

            is_match
        })?;

        self.stack.push(Value::Bool(result.unwrap_or_default()));
        Ok(())
    }

    #[inline]
    fn on_tuple<F, O>(&mut self, ty: TypeCheck, value: &Value, f: F) -> Result<Option<O>, VmError>
    where
        F: FnOnce(&[Value]) -> O,
    {
        use std::slice;

        Ok(match (ty, value) {
            (TypeCheck::Tuple, Value::Tuple(tuple)) => Some(f(&*tuple.borrow_ref()?)),
            (TypeCheck::Vec, Value::Vec(vec)) => Some(f(&*vec.borrow_ref()?)),
            (TypeCheck::Result(v), Value::Result(result)) => {
                let result = result.borrow_ref()?;

                Some(match (v, &*result) {
                    (0, Ok(ok)) => f(slice::from_ref(ok)),
                    (1, Err(err)) => f(slice::from_ref(err)),
                    _ => return Ok(None),
                })
            }
            (TypeCheck::Option(v), Value::Option(option)) => {
                let option = option.borrow_ref()?;

                Some(match (v, &*option) {
                    (0, Some(some)) => f(slice::from_ref(some)),
                    (1, None) => f(&[]),
                    _ => return Ok(None),
                })
            }
            (TypeCheck::GeneratorState(v), Value::GeneratorState(state)) => {
                use crate::GeneratorState::*;
                let state = state.borrow_ref()?;

                Some(match (v, &*state) {
                    (0, Complete(complete)) => f(slice::from_ref(complete)),
                    (1, Yielded(yielded)) => f(slice::from_ref(yielded)),
                    _ => return Ok(None),
                })
            }
            (TypeCheck::Type(hash), Value::TypedTuple(typed_tuple)) => {
                let typed_tuple = typed_tuple.borrow_ref()?;

                if typed_tuple.hash != hash {
                    return Ok(None);
                }

                Some(f(&*typed_tuple.tuple))
            }
            (TypeCheck::Variant(hash), Value::VariantTuple(variant_tuple)) => {
                let variant_tuple = variant_tuple.borrow_ref()?;

                if variant_tuple.hash != hash {
                    return Ok(None);
                }

                Some(f(&*variant_tuple.tuple))
            }
            (TypeCheck::Unit, Value::Unit) => Some(f(&[])),
            _ => None,
        })
    }

    #[inline]
    fn on_object_keys<F, O>(
        &mut self,
        type_check: TypeCheck,
        slot: usize,
        f: F,
    ) -> Result<Option<O>, VmError>
    where
        F: FnOnce(&Object<Value>, &[String]) -> O,
    {
        let value = self.stack.pop()?;

        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or_else(|| VmError::MissingStaticObjectKeys { slot })?;

        match (type_check, value) {
            (TypeCheck::Object, Value::Object(object)) => {
                let object = object.borrow_ref()?;
                return Ok(Some(f(&*object, keys)));
            }
            (TypeCheck::Type(hash), Value::TypedObject(typed_object)) => {
                let typed_object = typed_object.borrow_ref()?;

                if typed_object.hash == hash {
                    return Ok(Some(f(&typed_object.object, keys)));
                }
            }
            (TypeCheck::Variant(hash), Value::VariantObject(variant_object)) => {
                let variant_object = variant_object.borrow_ref()?;

                if variant_object.hash == hash {
                    return Ok(Some(f(&variant_object.object, keys)));
                }
            }
            _ => (),
        }

        Ok(None)
    }

    /// Construct a future from calling an async function.
    fn call_generator_fn(&mut self, offset: usize, args: usize) -> Result<(), VmError> {
        let stack = self.stack.drain_stack_top(args)?.collect::<Stack>();
        let mut vm = Self::new_with_stack(self.context.clone(), self.unit.clone(), stack);

        vm.ip = offset;

        let future = Generator::new(vm);
        self.stack.push(Value::Generator(Shared::new(future)));
        Ok(())
    }

    /// Construct a future from calling an async function.
    fn call_async_fn(&mut self, offset: usize, args: usize) -> Result<(), VmError> {
        let stack = self.stack.drain_stack_top(args)?.collect::<Stack>();
        let mut vm = Self::new_with_stack(self.context.clone(), self.unit.clone(), stack);

        vm.ip = offset;

        let future = Future::new(async move { vm.run().run_to_completion().await });

        self.stack.push(Value::Future(Shared::new(future)));
        Ok(())
    }

    fn call_offset_fn(
        &mut self,
        offset: usize,
        call: UnitFnCall,
        args: usize,
    ) -> Result<(), VmError> {
        match call {
            UnitFnCall::Immediate => {
                self.push_call_frame(offset, args)?;
            }
            UnitFnCall::Generator => {
                self.call_generator_fn(offset, args)?;
            }
            UnitFnCall::Async => {
                self.call_async_fn(offset, args)?;
            }
        }

        Ok(())
    }

    fn op_fn(&mut self, hash: Hash) -> Result<(), VmError> {
        let fn_ptr = match self.unit.lookup(hash) {
            Some(info) => {
                let args = info.signature.args;

                match &info.kind {
                    UnitFnKind::Offset { offset, call } => FnPtr::from_offset(
                        self.context.clone(),
                        self.unit.clone(),
                        *offset,
                        *call,
                        args,
                    ),
                    UnitFnKind::Tuple { hash } => FnPtr::from_tuple(*hash, args),
                    UnitFnKind::TupleVariant { enum_hash, hash } => {
                        FnPtr::from_variant_tuple(*enum_hash, *hash, args)
                    }
                }
            }
            None => {
                let handler = self
                    .context
                    .lookup(hash)
                    .ok_or_else(|| VmError::MissingFunction { hash })?;

                FnPtr::from_handler(handler.clone())
            }
        };

        self.stack.push(Value::FnPtr(Shared::new(fn_ptr)));
        Ok(())
    }

    /// Construct a closure on the top of the stack.
    fn op_closure(&mut self, hash: Hash, count: usize) -> Result<(), VmError> {
        let info = self
            .unit
            .lookup(hash)
            .ok_or_else(|| VmError::MissingFunction { hash })?;

        let args = info.signature.args;

        let (offset, call) = match &info.kind {
            UnitFnKind::Offset { offset, call } => (*offset, *call),
            _ => return Err(VmError::MissingFunction { hash }),
        };

        let environment = self.stack.pop_sequence(count)?;
        let environment = Shared::new(Tuple::from(environment));

        let fn_ptr = FnPtr::from_closure(
            self.context.clone(),
            self.unit.clone(),
            environment,
            offset,
            call,
            args,
        );
        self.stack.push(Value::FnPtr(Shared::new(fn_ptr)));
        Ok(())
    }

    /// Implementation of a function call.
    fn op_call(&mut self, hash: Hash, args: usize) -> Result<(), VmError> {
        match self.unit.lookup(hash) {
            Some(info) => {
                if info.signature.args != args {
                    return Err(VmError::ArgumentCountMismatch {
                        actual: args,
                        expected: info.signature.args,
                    });
                }

                match &info.kind {
                    UnitFnKind::Offset { offset, call } => {
                        let offset = *offset;
                        let call = *call;
                        self.call_offset_fn(offset, call, args)?;
                    }
                    UnitFnKind::Tuple { hash } => {
                        let hash = *hash;
                        let args = info.signature.args;
                        let value = self.allocate_typed_tuple(hash, args)?;
                        self.stack.push(value);
                    }
                    UnitFnKind::TupleVariant { enum_hash, hash } => {
                        let enum_hash = *enum_hash;
                        let hash = *hash;
                        let args = info.signature.args;
                        let value = self.allocate_tuple_variant(enum_hash, hash, args)?;
                        self.stack.push(value);
                    }
                }
            }
            None => {
                let handler = self
                    .context
                    .lookup(hash)
                    .ok_or_else(|| VmError::MissingFunction { hash })?;

                handler(&mut self.stack, args)?;
            }
        }

        Ok(())
    }

    #[inline]
    fn op_call_instance<H>(&mut self, hash: H, args: usize) -> Result<(), VmError>
    where
        H: IntoTypeHash,
    {
        // NB: +1 to include the instance itself.
        let args = args + 1;
        let instance = self.stack.peek().ok_or_else(|| StackError::StackEmpty)?;
        let value_type = instance.value_type()?;
        let hash = Hash::instance_function(value_type, hash);

        match self.unit.lookup(hash) {
            Some(info) => {
                if info.signature.args != args {
                    return Err(VmError::ArgumentCountMismatch {
                        actual: args,
                        expected: info.signature.args,
                    });
                }

                match info.kind {
                    UnitFnKind::Offset { offset, call } => {
                        self.call_offset_fn(offset, call, args)?;
                    }
                    _ => {
                        return Err(VmError::MissingInstanceFunction {
                            instance: instance.type_info()?,
                            hash,
                        });
                    }
                }
            }
            None => {
                let handler = match self.context.lookup(hash) {
                    Some(handler) => handler,
                    None => {
                        return Err(VmError::MissingInstanceFunction {
                            instance: instance.type_info()?,
                            hash,
                        });
                    }
                };

                handler(&mut self.stack, args)?;
            }
        }

        Ok(())
    }

    async fn op_call_fn(&mut self, args: usize) -> Result<(), VmError> {
        let function = self.stack.pop()?;

        let hash = match function {
            Value::Type(hash) => hash,
            Value::FnPtr(fn_ptr) => {
                let fn_ptr = fn_ptr.owned_ref()?;
                fn_ptr.call_with_vm(self, args).await?;
                return Ok(());
            }
            actual => {
                let actual_type = actual.type_info()?;
                return Err(VmError::UnsupportedCallFn { actual_type });
            }
        };

        self.op_call(hash, args)?;
        Ok(())
    }

    /// Advance the instruction pointer.
    fn advance(&mut self) {
        self.ip = self.ip.overflowing_add(1).0;
    }

    /// Evaluate a single instruction.
    pub(crate) async fn run_for(
        &mut self,
        mut limit: Option<usize>,
    ) -> Result<StopReason, VmError> {
        loop {
            let inst = *self
                .unit
                .instruction_at(self.ip)
                .ok_or_else(|| VmError::IpOutOfBounds)?;

            log::trace!("{}: {}", self.ip, inst);

            match inst {
                Inst::Not => {
                    self.op_not()?;
                }
                Inst::Add => {
                    self.op_add()?;
                }
                Inst::AddAssign { offset } => {
                    self.op_add_assign(offset)?;
                }
                Inst::Sub => {
                    self.op_sub()?;
                }
                Inst::SubAssign { offset } => {
                    self.op_sub_assign(offset)?;
                }
                Inst::Mul => {
                    self.op_mul()?;
                }
                Inst::MulAssign { offset } => {
                    self.op_mul_assign(offset)?;
                }
                Inst::Div => {
                    self.op_div()?;
                }
                Inst::DivAssign { offset } => {
                    self.op_div_assign(offset)?;
                }
                Inst::Rem => {
                    self.op_rem()?;
                }
                Inst::Fn { hash } => {
                    self.op_fn(hash)?;
                }
                Inst::Closure { hash, count } => {
                    self.op_closure(hash, count)?;
                }
                Inst::Call { hash, args } => {
                    self.op_call(hash, args)?;
                }
                Inst::CallInstance { hash, args } => {
                    self.op_call_instance(hash, args)?;
                }
                Inst::CallFn { args } => {
                    self.op_call_fn(args).await?;
                }
                Inst::LoadInstanceFn { hash } => {
                    self.op_load_instance_fn(hash)?;
                }
                Inst::IndexGet => {
                    self.op_index_get()?;
                }
                Inst::TupleIndexGet { index } => {
                    self.op_tuple_index_get(index)?;
                }
                Inst::TupleIndexSet { index } => {
                    self.op_tuple_index_set(index)?;
                }
                Inst::TupleIndexGetAt { offset, index } => {
                    self.op_tuple_index_get_at(offset, index)?;
                }
                Inst::ObjectSlotIndexGet { slot } => {
                    self.op_object_slot_index_get(slot)?;
                }
                Inst::ObjectSlotIndexGetAt { offset, slot } => {
                    self.op_object_slot_index_get_at(offset, slot)?;
                }
                Inst::IndexSet => {
                    self.op_index_set()?;
                }
                Inst::Return => {
                    if self.op_return()? {
                        self.advance();
                        return Ok(StopReason::Exited);
                    }
                }
                Inst::ReturnUnit => {
                    if self.op_return_unit()? {
                        self.advance();
                        return Ok(StopReason::Exited);
                    }
                }
                Inst::Await => {
                    self.op_await().await?;
                }
                Inst::Select { len } => {
                    self.op_select(len).await?;
                }
                Inst::Pop => {
                    self.stack.pop()?;
                }
                Inst::PopN { count } => {
                    self.op_popn(count)?;
                }
                Inst::PopAndJumpIf { count, offset } => {
                    self.op_pop_and_jump_if(count, offset)?;
                }
                Inst::PopAndJumpIfNot { count, offset } => {
                    self.op_pop_and_jump_if_not(count, offset)?;
                }
                Inst::Clean { count } => {
                    self.op_clean(count)?;
                }
                Inst::Integer { number } => {
                    self.stack.push(Value::Integer(number));
                }
                Inst::Float { number } => {
                    self.stack.push(Value::Float(number));
                }
                Inst::Copy { offset } => {
                    self.op_copy(offset)?;
                }
                Inst::Drop { offset } => {
                    self.op_drop(offset)?;
                }
                Inst::Dup => {
                    self.op_dup()?;
                }
                Inst::Replace { offset } => {
                    self.op_replace(offset)?;
                }
                Inst::Gt => {
                    self.op_gt()?;
                }
                Inst::Gte => {
                    self.op_gte()?;
                }
                Inst::Lt => {
                    self.op_lt()?;
                }
                Inst::Lte => {
                    self.op_lte()?;
                }
                Inst::Eq => {
                    self.op_eq()?;
                }
                Inst::Neq => {
                    self.op_neq()?;
                }
                Inst::Jump { offset } => {
                    self.op_jump(offset)?;
                }
                Inst::JumpIf { offset } => {
                    self.op_jump_if(offset)?;
                }
                Inst::JumpIfNot { offset } => {
                    self.op_jump_if_not(offset)?;
                }
                Inst::JumpIfBranch { branch, offset } => {
                    self.op_jump_if_branch(branch, offset)?;
                }
                Inst::Unit => {
                    self.stack.push(Value::Unit);
                }
                Inst::Bool { value } => {
                    self.stack.push(Value::Bool(value));
                }
                Inst::Vec { count } => {
                    self.op_vec(count)?;
                }
                Inst::Tuple { count } => {
                    self.op_tuple(count)?;
                }
                Inst::Object { slot } => {
                    self.op_object(slot)?;
                }
                Inst::TypedObject { hash, slot } => {
                    self.op_typed_object(hash, slot)?;
                }
                Inst::VariantObject {
                    enum_hash,
                    hash,
                    slot,
                } => {
                    self.op_variant_object(enum_hash, hash, slot)?;
                }
                Inst::Type { hash } => {
                    self.stack.push(Value::Type(hash));
                }
                Inst::Char { c } => {
                    self.stack.push(Value::Char(c));
                }
                Inst::Byte { b } => {
                    self.stack.push(Value::Byte(b));
                }
                Inst::String { slot } => {
                    self.op_string(slot)?;
                }
                Inst::Bytes { slot } => {
                    self.op_bytes(slot)?;
                }
                Inst::StringConcat { len, size_hint } => {
                    self.op_string_concat(len, size_hint)?;
                }
                Inst::Is => {
                    self.op_is()?;
                }
                Inst::IsNot => {
                    self.op_is_not()?;
                }
                Inst::IsUnit => {
                    self.op_is_unit()?;
                }
                Inst::IsValue => {
                    self.op_is_value()?;
                }
                Inst::Unwrap => {
                    self.op_unwrap()?;
                }
                Inst::And => {
                    self.op_and()?;
                }
                Inst::Or => {
                    self.op_or()?;
                }
                Inst::EqByte { byte } => {
                    self.op_eq_byte(byte)?;
                }
                Inst::EqCharacter { character } => {
                    self.op_eq_character(character)?;
                }
                Inst::EqInteger { integer } => {
                    self.op_eq_integer(integer)?;
                }
                Inst::EqStaticString { slot } => {
                    self.op_eq_static_string(slot)?;
                }
                Inst::MatchSequence {
                    type_check,
                    len,
                    exact,
                } => {
                    self.op_match_sequence(type_check, len, exact)?;
                }
                Inst::MatchObject {
                    type_check,
                    slot,
                    exact,
                } => {
                    self.op_match_object(type_check, slot, exact)?;
                }
                Inst::Yield => {
                    self.advance();
                    return Ok(StopReason::Yielded);
                }
                Inst::YieldUnit => {
                    self.advance();
                    self.stack.push(Value::Unit);
                    return Ok(StopReason::Yielded);
                }
                Inst::Panic { reason } => {
                    return Err(VmError::Panic {
                        reason: Panic::from(reason),
                    });
                }
            }

            self.advance();

            if let Some(limit) = &mut limit {
                if *limit <= 1 {
                    return Ok(StopReason::Limited);
                }

                *limit -= 1;
            }
        }
    }
}

/// The reason why the virtual machine execution stopped.
#[derive(Debug, Clone, Copy)]
pub enum StopReason {
    /// The virtual machine exited by running out of call frames.
    Exited,
    /// The virtual machine exited because it ran out of execution quota.
    Limited,
    /// The virtual machine yielded.
    Yielded,
}

impl fmt::Display for StopReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exited => write!(f, "exited"),
            Self::Limited => write!(f, "limited"),
            Self::Yielded => write!(f, "yielded"),
        }
    }
}

/// The task of a unit being run.
pub struct Task<'a, T> {
    /// The virtual machine associated with the task.
    vm: &'a mut Vm,
    /// Marker holding output type.
    _marker: marker::PhantomData<&'a mut T>,
}

impl<'a, T> Task<'a, T>
where
    T: FromValue,
{
    /// Get access to the underlying virtual machine.
    pub fn vm(&self) -> &Vm {
        self.vm
    }

    /// Get access to the used compilation unit.
    pub fn unit(&self) -> &Unit {
        &*self.vm.unit
    }

    /// Run the given task to completion.
    async fn inner_run_to_completion(&mut self) -> Result<T, VmError> {
        match self.vm.run_for(None).await {
            Ok(StopReason::Exited) => (),
            Ok(reason) => return Err(VmError::Stopped { reason }),
            Err(e) => return Err(e),
        }

        let value = self.vm.pop_decode()?;
        debug_assert!(self.vm.stack.is_empty());
        Ok(value)
    }

    /// Run to completion implementation to use internally.
    pub async fn run_to_completion(&mut self) -> Result<T, VmError> {
        match self.inner_run_to_completion().await {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_unwinded(self.vm.ip())),
        }
    }

    /// Step the given task until the return value is available.
    pub async fn step(&mut self) -> Result<Option<T>, VmError> {
        match self.vm.run_for(Some(1)).await? {
            StopReason::Limited => return Ok(None),
            StopReason::Exited => (),
            reason => return Err(VmError::Stopped { reason }),
        }

        let value = self.vm.pop_decode()?;
        debug_assert!(self.vm.stack.is_empty());
        Ok(Some(value))
    }
}
