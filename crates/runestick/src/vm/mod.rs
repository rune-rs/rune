use crate::bytes::Bytes;
use crate::context::Context;
use crate::future::Future;
use crate::future::SelectFuture;
use crate::hash::Hash;
use crate::item::Component;
use crate::panic::Panic;
use crate::reflection::{FromValue, UnsafeIntoArgs};
use crate::shared::{Shared, StrongMut};
use crate::stack::{Stack, StackError};
use crate::unit::{CompilationUnit, UnitFnCall, UnitFnKind};
use crate::value::{
    Integer, Object, TypedObject, TypedTuple, Value, ValueError, ValueTypeInfo, VariantObject,
    VariantTuple,
};
use std::any;
use std::fmt;
use std::marker;
use std::mem;
use std::rc::Rc;
use thiserror::Error;

pub(crate) mod inst;

pub use self::inst::{Inst, OptionVariant, PanicReason, ResultVariant, TypeCheck};
use crate::access::AccessError;

/// Errors raised by the execution of the virtual machine.
#[derive(Debug, Error)]
pub enum VmError {
    /// The virtual machine panicked for a specific reason.
    #[error("panicked `{reason}`")]
    Panic {
        /// The reason for the panic.
        reason: Panic,
    },
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
        #[from]
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
    /// Unsupported argument to string-concat
    #[error("unsupported string-concat argument `{actual}`")]
    UnsupportedStringConcatArgument {
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
    #[error(
        "the index set operation `{target_type}[{index_type}] = {value_type}` is not supported"
    )]
    UnsupportedIndexSet {
        /// The target type to set.
        target_type: ValueTypeInfo,
        /// The index to set.
        index_type: ValueTypeInfo,
        /// The value to set.
        value_type: ValueTypeInfo,
    },
    /// An index get operation that is not supported.
    #[error("the index get operation `{target_type}[{index_type}]` is not supported")]
    UnsupportedIndexGet {
        /// The target type to get.
        target_type: ValueTypeInfo,
        /// The index to get.
        index_type: ValueTypeInfo,
    },
    /// A vector index get operation that is not supported.
    #[error("the vector index get operation is not supported on `{target_type}`")]
    UnsupportedVecIndexGet {
        /// The target type we tried to perform the vector indexing on.
        target_type: ValueTypeInfo,
    },
    /// An tuple index get operation that is not supported.
    #[error("the tuple index get operation is not supported on `{target_type}`")]
    UnsupportedTupleIndexGet {
        /// The target type we tried to perform the tuple indexing on.
        target_type: ValueTypeInfo,
    },
    /// An object slot index get operation that is not supported.
    #[error("the object slot index get operation on `{target_type}` is not supported")]
    UnsupportedObjectSlotIndexGet {
        /// The target type we tried to perform the object indexing on.
        target_type: ValueTypeInfo,
    },
    /// An is operation is not supported.
    #[error("`{value_type} is {test_type}` is not supported")]
    UnsupportedIs {
        /// The argument that is not supported.
        value_type: ValueTypeInfo,
        /// The type that is not supported.
        test_type: ValueTypeInfo,
    },
    /// Encountered a value that could not be dereferenced.
    #[error("replace deref `*{target_type} = {value_type}` is not supported")]
    UnsupportedReplaceDeref {
        /// The type we try to assign to.
        target_type: ValueTypeInfo,
        /// The type we try to assign.
        value_type: ValueTypeInfo,
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
}

/// Generate a primitive combination of operations.
macro_rules! primitive_ops {
    ($vm:expr, $a:ident $op:tt $b:ident) => {
        match ($a, $b) {
            (Value::Char($a), Value::Char($b)) => $a $op $b,
            (Value::Bool($a), Value::Bool($b)) => $a $op $b,
            (Value::Integer($a), Value::Integer($b)) => $a $op $b,
            (Value::Float($a), Value::Float($b)) => $a $op $b,
            (lhs, rhs) => return Err(VmError::UnsupportedBinaryOperation {
                op: stringify!($op),
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }),
        }
    }
}

/// Generate a boolean combination of operations.
macro_rules! boolean_ops {
    ($vm:expr, $a:ident $op:tt $b:ident) => {
        match ($a, $b) {
            (Value::Bool($a), Value::Bool($b)) => $a $op $b,
            (lhs, rhs) => return Err(VmError::UnsupportedBinaryOperation {
                op: stringify!($op),
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }),
        }
    }
}

macro_rules! check_float {
    ($float:expr, $error:ident) => {
        if !$float.is_finite() {
            return Err(VmError::$error);
        } else {
            $float
        }
    };
}

/// Generate a primitive combination of operations.
macro_rules! numeric_ops {
    ($vm:expr, $context:expr, $fn:expr, $op:tt, $a:ident . $checked_op:ident ( $b:ident ), $error:ident) => {
        match ($a, $b) {
            (Value::Integer($a), Value::Integer($b)) => {
                $vm.stack.push(Value::Integer({
                    match $a.$checked_op($b) {
                        Some(value) => value,
                        None => return Err(VmError::$error),
                    }
                }));
            },
            (Value::Float($a), Value::Float($b)) => {
                $vm.stack.push(Value::Float(check_float!($a $op $b, $error)));
            },
            (lhs, rhs) => {
                let ty = lhs.value_type()?;
                let hash = Hash::instance_function(ty, *$fn);

                let handler = match $context.lookup(hash) {
                    Some(handler) => handler,
                    None => {
                        return Err(VmError::UnsupportedBinaryOperation {
                            op: stringify!($op),
                            lhs: lhs.type_info()?,
                            rhs: rhs.type_info()?,
                        });
                    }
                };

                $vm.stack.push(rhs);
                $vm.stack.push(lhs);
                handler(&mut $vm.stack, 1)?;
            },
        }
    }
}

/// Generate a primitive combination of operations.
macro_rules! assign_ops {
    ($vm:expr, $context:expr, $hash:expr, $op:tt, $a:ident . $checked_op:ident ( $b:ident ), $error:ident) => {
        match ($a, $b) {
            (Value::Integer($a), Value::Integer($b)) => Value::Integer({
                match $a.$checked_op($b) {
                    Some(value) => value,
                    None => return Err(VmError::$error),
                }
            }),
            (Value::Float($a), Value::Float($b)) => Value::Float({
                check_float!($a $op $b, $error)
            }),
            (lhs, rhs) => {
                if !$vm.call_instance_fn($context, *$hash, &lhs, (&rhs,))? {
                    return Err(VmError::UnsupportedBinaryOperation {
                        op: stringify!($op),
                        lhs: lhs.type_info()?,
                        rhs: rhs.type_info()?,
                    });
                }

                $vm.stack.pop()?;
                lhs
            }
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
    /// The current instruction pointer.
    ip: usize,
    /// The current stack.
    stack: Stack,
    /// We have exited from the last frame.
    exited: bool,
    /// Frames relative to the stack.
    call_frames: Vec<CallFrame>,
    /// The `branch` registry used for certain operations.
    branch: Option<usize>,
}

impl Vm {
    /// Construct a new runestick virtual machine.
    pub const fn new() -> Self {
        Self {
            ip: 0,
            stack: Stack::new(),
            exited: false,
            call_frames: Vec::new(),
            branch: None,
        }
    }

    /// Reset this virtual machine, freeing all memory used.
    ///
    /// # Safety
    ///
    /// Any unsafe references constructed through the following methods:
    /// * [StrongMut::into_raw]
    /// * [Ref::unsafe_into_ref]
    ///
    /// Must not outlive a call to clear, nor this virtual machine.
    pub fn clear(&mut self) {
        self.ip = 0;
        self.exited = false;
        self.stack.clear();
        self.call_frames.clear();
    }

    /// Access the current instruction pointer.
    pub fn ip(&self) -> usize {
        self.ip
    }

    /// Modify the current instruction pointer.
    pub fn modify_ip(&mut self, offset: isize) -> Result<(), VmError> {
        let ip = if offset < 0 {
            self.ip.checked_sub(-offset as usize)
        } else {
            self.ip.checked_add(offset as usize)
        };

        self.ip = ip.ok_or_else(|| VmError::IpOutOfBounds)?;
        Ok(())
    }

    /// Iterate over the stack, producing the value associated with each stack
    /// item.
    pub fn iter_stack_debug(&self) -> impl Iterator<Item = &Value> + '_ {
        self.stack.iter()
    }

    /// Call the given function in the given compilation unit.
    pub fn call_function<'a, A: 'a, T, I>(
        &'a mut self,
        unit: Rc<CompilationUnit>,
        context: Rc<Context>,
        name: I,
        args: A,
    ) -> Result<Task<'a, T>, VmError>
    where
        I: IntoIterator,
        I::Item: AsRef<Component>,
        A: 'a + UnsafeIntoArgs,
        T: FromValue,
    {
        let hash = Hash::function(name);

        let function = unit
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
        unsafe {
            args.unsafe_into_args(&mut self.stack)?;
        }

        Ok(Task {
            vm: self,
            unit,
            context,
            _marker: marker::PhantomData,
        })
    }

    /// Run the given program on the virtual machine.
    pub fn run<'a, T>(&'a mut self, unit: Rc<CompilationUnit>, context: Rc<Context>) -> Task<'a, T>
    where
        T: FromValue,
    {
        Task {
            vm: self,
            unit,
            context,
            _marker: marker::PhantomData,
        }
    }

    async fn op_await(&mut self) -> Result<(), VmError> {
        let future = self.stack.pop()?.into_future()?;
        let mut future = future.strong_mut()?;
        let value = (&mut *future).await?;
        self.stack.push(value);
        Ok(())
    }

    async fn op_select(&mut self, len: usize) -> Result<(), VmError> {
        use futures::stream::StreamExt as _;

        let (branch, value) = {
            let mut futures = futures::stream::FuturesUnordered::new();
            let mut guards = Vec::new();

            for index in 0..len {
                let future = self.stack.pop()?.into_future()?;
                let future = future.strong_mut()?;

                if future.is_completed() {
                    continue;
                }

                // Safety: we have exclusive access to the virtual machine, so we
                // can assert that nothing is invalidate for the duration of this
                // select.
                unsafe {
                    let (raw_future, guard) = StrongMut::into_raw(future);
                    futures.push(SelectFuture::new_unchecked(raw_future, index));
                    guards.push(guard);
                };
            }

            // NB: nothing to poll.
            if futures.is_empty() {
                return Ok(());
            }

            let result = futures.next().await.unwrap();
            let (index, value) = result?;
            drop(guards);
            (index, value)
        };

        self.stack.push(value);
        self.branch = Some(branch);
        Ok(())
    }

    fn call_instance_fn<A>(
        &mut self,
        context: &Context,
        hash: Hash,
        instance: &Value,
        args: A,
    ) -> Result<bool, VmError>
    where
        A: UnsafeIntoArgs,
    {
        let ty = instance.value_type()?;
        let hash = Hash::instance_function(ty, hash);

        let handler = match context.lookup(hash) {
            Some(handler) => handler,
            None => return Ok(false),
        };

        // Safety: This function can only be called inside the virtual machine,
        // which is guaranteed to not outlive the stack. Allowing us to safely
        // encode reference into it.
        unsafe {
            args.unsafe_into_args(&mut self.stack)?;
        }

        self.stack.push(instance.clone());
        handler(&mut self.stack, 1)?;
        Ok(true)
    }

    /// Pop a number of values from the stack.
    fn op_popn(&mut self, n: usize) -> Result<(), VmError> {
        self.stack.popn(n)?;
        Ok(())
    }

    /// pop-and-jump-if instruction.
    fn op_pop_and_jump_if(
        &mut self,
        count: usize,
        offset: isize,
        update_ip: &mut bool,
    ) -> Result<(), VmError> {
        if !self.stack.pop()?.into_bool()? {
            return Ok(());
        }

        self.stack.popn(count)?;
        self.modify_ip(offset)?;
        *update_ip = false;
        Ok(())
    }

    /// pop-and-jump-if-not instruction.
    fn op_pop_and_jump_if_not(
        &mut self,
        count: usize,
        offset: isize,
        update_ip: &mut bool,
    ) -> Result<(), VmError> {
        if self.stack.pop()?.into_bool()? {
            return Ok(());
        }

        self.stack.popn(count)?;
        self.modify_ip(offset)?;
        *update_ip = false;
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

    fn op_gt(&mut self) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        self.stack.push(Value::Bool(primitive_ops!(self, a > b)));
        Ok(())
    }

    fn op_gte(&mut self) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        self.stack.push(Value::Bool(primitive_ops!(self, a >= b)));
        Ok(())
    }

    fn op_lt(&mut self) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        self.stack.push(Value::Bool(primitive_ops!(self, a < b)));
        Ok(())
    }

    fn op_lte(&mut self) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        self.stack.push(Value::Bool(primitive_ops!(self, a <= b)));
        Ok(())
    }

    /// Push a new call frame.
    fn push_call_frame(&mut self, new_ip: usize, args: usize) -> Result<(), VmError> {
        let stack_top = self.stack.push_stack_top(args)?;

        self.call_frames.push(CallFrame {
            ip: self.ip,
            stack_top,
        });

        self.ip = new_ip;
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
    ///
    /// Note: External types are compared by their slot, but should eventually
    /// use a dynamically resolve equality function.
    fn value_ptr_eq(&self, a: &Value, b: &Value) -> Result<bool, VmError> {
        Ok(match (a, b) {
            (Value::Unit, Value::Unit) => true,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Vec(a), Value::Vec(b)) => {
                let a = a.get_ref()?;
                let b = b.get_ref()?;

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
                let a = a.get_ref()?;
                let b = b.get_ref()?;

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
                let a = a.get_ref()?;
                let b = b.get_ref()?;
                *a == *b
            }
            (Value::StaticString(a), Value::String(b)) => {
                let b = b.get_ref()?;
                ***a == *b
            }
            (Value::String(a), Value::StaticString(b)) => {
                let a = a.get_ref()?;
                *a == ***b
            }
            // fast string comparison: exact string slot.
            (Value::StaticString(a), Value::StaticString(b)) => a == b,
            // fast external comparison by slot.
            // TODO: implement ptr equals.
            // (Value::External(a), Value::External(b)) => a == b,
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
    fn op_jump(&mut self, offset: isize, update_ip: &mut bool) -> Result<(), VmError> {
        self.modify_ip(offset)?;
        *update_ip = false;
        Ok(())
    }

    /// Perform a conditional jump operation.
    #[inline]
    fn op_jump_if(&mut self, offset: isize, update_ip: &mut bool) -> Result<(), VmError> {
        if self.stack.pop()?.into_bool()? {
            self.modify_ip(offset)?;
            *update_ip = false;
        }

        Ok(())
    }

    /// Perform a conditional jump operation.
    #[inline]
    fn op_jump_if_not(&mut self, offset: isize, update_ip: &mut bool) -> Result<(), VmError> {
        if !self.stack.pop()?.into_bool()? {
            self.modify_ip(offset)?;
            *update_ip = false;
        }

        Ok(())
    }

    /// Perform a branch-conditional jump operation.
    #[inline]
    fn op_jump_if_branch(
        &mut self,
        branch: usize,
        offset: isize,
        update_ip: &mut bool,
    ) -> Result<(), VmError> {
        if let Some(current) = self.branch {
            if current == branch {
                self.branch = None;
                self.modify_ip(offset)?;
                *update_ip = false;
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

        let tuple = tuple.into_boxed_slice();
        let value = Value::Tuple(Shared::new(tuple));
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

    #[inline]
    fn op_add(&mut self, context: &Rc<Context>) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        numeric_ops!(self, context, crate::ADD, +, a.checked_add(b), Overflow);
        Ok(())
    }

    #[inline]
    fn op_sub(&mut self, context: &Rc<Context>) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        numeric_ops!(self, context, crate::SUB, -, a.checked_sub(b), Underflow);
        Ok(())
    }

    #[inline]
    fn op_mul(&mut self, context: &Rc<Context>) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        numeric_ops!(self, context, crate::MUL, *, a.checked_mul(b), Overflow);
        Ok(())
    }

    #[inline]
    fn op_div(&mut self, context: &Rc<Context>) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        numeric_ops!(self, context, crate::DIV, /, a.checked_div(b), DivideByZero);
        Ok(())
    }

    /// Perform an index set operation.
    #[inline]
    fn op_index_set(&mut self, context: &Rc<Context>) -> Result<(), VmError> {
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
                    local_field = string.get_ref()?;
                    local_field.as_str()
                }
                Value::StaticString(string) => string.as_ref(),
                _ => break,
            };

            match &target {
                Value::Object(object) => {
                    let mut object = object.get_mut()?;
                    object.insert(field.to_owned(), value);
                    return Ok(());
                }
                Value::TypedObject(typed_object) => {
                    let mut typed_object = typed_object.get_mut()?;

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
                    let mut variant_object = variant_object.get_mut()?;

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

        let ty = target.value_type()?;
        let hash = Hash::instance_function(ty, *crate::INDEX_SET);

        let handler = match context.lookup(hash) {
            Some(handler) => handler,
            None => {
                let target_type = target.type_info()?;
                let index_type = index.type_info()?;
                let value_type = value.type_info()?;

                return Err(VmError::UnsupportedIndexSet {
                    target_type,
                    index_type,
                    value_type,
                });
            }
        };

        self.stack.push(value);
        self.stack.push(index);
        self.stack.push(target);
        handler(&mut self.stack, 2)?;
        Ok(())
    }

    #[inline]
    fn op_return(&mut self) -> Result<(), VmError> {
        let return_value = self.stack.pop()?;
        self.exited = self.pop_call_frame()?;
        self.stack.push(return_value);
        Ok(())
    }

    #[inline]
    fn op_return_unit(&mut self) -> Result<(), VmError> {
        self.exited = self.pop_call_frame()?;
        self.stack.push(Value::Unit);
        Ok(())
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
            Value::Object(target) => target.get_ref()?.get(field).cloned(),
            Value::TypedObject(target) => target.get_ref()?.object.get(field).cloned(),
            Value::VariantObject(target) => target.get_ref()?.object.get(field).cloned(),
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
    fn try_tuple_like_index_get(&mut self, target: &Value, index: usize) -> Result<bool, VmError> {
        let value = match target {
            Value::Unit => None,
            Value::Tuple(tuple) => tuple.get_ref()?.get(index).cloned(),
            Value::Vec(vec) => vec.get_ref()?.get(index).cloned(),
            Value::Result(result) => {
                let result = result.get_ref()?;

                match &*result {
                    Ok(ok) if index == 0 => Some(ok.clone()),
                    Err(err) if index == 0 => Some(err.clone()),
                    _ => None,
                }
            }
            Value::Option(option) => {
                let option = option.get_ref()?;

                match &*option {
                    Some(some) if index == 0 => Some(some.clone()),
                    _ => None,
                }
            }
            Value::TypedTuple(typed_tuple) => {
                let typed_tuple = typed_tuple.get_ref()?;
                typed_tuple.tuple.get(index).cloned()
            }
            Value::VariantTuple(variant_tuple) => {
                let variant_tuple = variant_tuple.get_ref()?;
                variant_tuple.tuple.get(index).cloned()
            }
            _ => return Ok(false),
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

        self.stack.push(value);
        Ok(true)
    }

    /// Perform an index get operation.
    #[inline]
    fn op_index_get(&mut self, context: &Rc<Context>) -> Result<(), VmError> {
        let target = self.stack.pop()?;
        let index = self.stack.pop()?;

        // This is a useful pattern.
        #[allow(clippy::never_loop)]
        loop {
            match &index {
                Value::String(string) => {
                    let string_ref = string.get_ref()?;

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

                    if self.try_tuple_like_index_get(&target, index)? {
                        return Ok(());
                    }
                }
                _ => break,
            };
        }

        let ty = target.value_type()?;
        let hash = Hash::instance_function(ty, *crate::INDEX_GET);

        let handler = match context.lookup(hash) {
            Some(handler) => handler,
            None => {
                let target_type = target.type_info()?;
                let index_type = index.type_info()?;

                return Err(VmError::UnsupportedIndexGet {
                    target_type,
                    index_type,
                });
            }
        };

        self.stack.push(index);
        self.stack.push(target);
        handler(&mut self.stack, 1)?;
        Ok(())
    }

    /// Perform an index get operation.
    #[inline]
    fn op_vec_index_get(&mut self, index: usize) -> Result<(), VmError> {
        let target = self.stack.pop()?;

        let value = match target {
            Value::Vec(vec) => {
                let vec = vec.get_ref()?;

                match vec.get(index).cloned() {
                    Some(value) => value,
                    None => {
                        return Err(VmError::VecIndexMissing { index });
                    }
                }
            }
            target_type => {
                let target_type = target_type.type_info()?;
                return Err(VmError::UnsupportedVecIndexGet { target_type });
            }
        };

        self.stack.push(value);
        Ok(())
    }

    /// Perform an index get operation specialized for tuples.
    #[inline]
    fn op_tuple_index_get(&mut self, index: usize) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        if !self.try_tuple_like_index_get(&value, index)? {
            return Err(VmError::UnsupportedTupleIndexGet {
                target_type: value.type_info()?,
            });
        }

        Ok(())
    }

    /// Perform a specialized index get operation on an object.
    #[inline]
    fn op_object_slot_index_get(
        &mut self,
        unit: &Rc<CompilationUnit>,
        string_slot: usize,
    ) -> Result<(), VmError> {
        let target = self.stack.pop()?;

        let value = match target {
            Value::Object(object) => {
                let index = unit.lookup_string(string_slot)?;
                let object = object.get_ref()?;

                match object.get(&***index).cloned() {
                    Some(value) => value,
                    None => {
                        return Err(VmError::ObjectIndexMissing { slot: string_slot });
                    }
                }
            }
            Value::TypedObject(typed_object) => {
                let index = unit.lookup_string(string_slot)?;
                let typed_object = typed_object.get_ref()?;

                match typed_object.object.get(&***index).cloned() {
                    Some(value) => value,
                    None => {
                        return Err(VmError::ObjectIndexMissing { slot: string_slot });
                    }
                }
            }
            Value::VariantObject(variant_object) => {
                let index = unit.lookup_string(string_slot)?;
                let variant_object = variant_object.get_ref()?;

                match variant_object.object.get(&***index).cloned() {
                    Some(value) => value,
                    None => {
                        return Err(VmError::ObjectIndexMissing { slot: string_slot });
                    }
                }
            }
            target_type => {
                let target_type = target_type.type_info()?;
                return Err(VmError::UnsupportedObjectSlotIndexGet { target_type });
            }
        };

        self.stack.push(value);
        Ok(())
    }

    /// Operation to allocate an object.
    #[inline]
    fn op_object(&mut self, unit: &Rc<CompilationUnit>, slot: usize) -> Result<(), VmError> {
        let keys = unit
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
    fn op_typed_object(
        &mut self,
        unit: &Rc<CompilationUnit>,
        hash: Hash,
        slot: usize,
    ) -> Result<(), VmError> {
        let keys = unit
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
        unit: &Rc<CompilationUnit>,
        enum_hash: Hash,
        hash: Hash,
        slot: usize,
    ) -> Result<(), VmError> {
        let keys = unit
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
    fn op_string(&mut self, unit: &Rc<CompilationUnit>, slot: usize) -> Result<(), VmError> {
        let string = unit.lookup_string(slot)?;
        let value = Value::StaticString(string.clone());
        self.stack.push(value);
        Ok(())
    }

    /// Optimize operation to perform string concatenation.
    #[inline]
    fn op_string_concat(
        &mut self,
        context: &Rc<Context>,
        len: usize,
        size_hint: usize,
    ) -> Result<(), VmError> {
        let mut buf = String::with_capacity(size_hint);

        for _ in 0..len {
            let value = self.stack.pop()?;

            match value {
                Value::String(string) => {
                    buf.push_str(&*string.get_ref()?);
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
                    if !self.call_instance_fn(context, *crate::FMT_DISPLAY, &actual, (&mut buf,))? {
                        return Err(VmError::UnsupportedStringConcatArgument {
                            actual: actual.type_info()?,
                        });
                    }

                    self.stack.pop()?;
                }
            }
        }

        self.stack.push(Value::String(Shared::new(buf)));
        return Ok(());
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
    fn is_instance(
        &mut self,
        unit: &Rc<CompilationUnit>,
        context: &Rc<Context>,
    ) -> Result<bool, VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;

        let hash = match b {
            Value::Type(hash) => hash,
            _ => {
                return Err(VmError::UnsupportedIs {
                    value_type: a.type_info()?,
                    test_type: b.type_info()?,
                });
            }
        };

        let is_instance = match a {
            Value::TypedObject(typed_object) => typed_object.get_ref()?.hash == hash,
            Value::TypedTuple(typed_tuple) => typed_tuple.get_ref()?.hash == hash,
            Value::VariantObject(variant_object) => variant_object.get_ref()?.enum_hash == hash,
            Value::VariantTuple(variant_tuple) => variant_tuple.get_ref()?.enum_hash == hash,
            Value::Option(..) => {
                let option_type = context
                    .option_type()
                    .ok_or_else(|| VmError::MissingType { hash })?;

                option_type == hash
            }
            Value::Result(..) => {
                let result_type = context
                    .result_type()
                    .ok_or_else(|| VmError::MissingType { hash })?;

                result_type == hash
            }
            a => {
                let value_type = match unit.lookup_type(hash) {
                    Some(info) => info.value_type,
                    None => {
                        context
                            .lookup_type(hash)
                            .ok_or_else(|| VmError::MissingType { hash })?
                            .value_type
                    }
                };

                a.value_type()? == value_type
            }
        };

        Ok(is_instance)
    }

    #[inline]
    fn op_is(&mut self, unit: &Rc<CompilationUnit>, context: &Rc<Context>) -> Result<(), VmError> {
        let is_instance = self.is_instance(unit, context)?;
        self.stack.push(Value::Bool(is_instance));
        Ok(())
    }

    #[inline]
    fn op_is_not(
        &mut self,
        unit: &Rc<CompilationUnit>,
        context: &Rc<Context>,
    ) -> Result<(), VmError> {
        let is_instance = self.is_instance(unit, context)?;
        self.stack.push(Value::Bool(!is_instance));
        Ok(())
    }

    /// Test if the top of the stack is an error.
    #[inline]
    fn op_is_value(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let is_value = match value {
            Value::Result(result) => result.get_ref()?.is_ok(),
            Value::Option(option) => option.get_ref()?.is_some(),
            other => {
                return Err(VmError::UnsupportedIsValueOperand {
                    actual: other.type_info()?,
                })
            }
        };

        self.stack.push(Value::Bool(is_value));
        Ok(())
    }

    /// Operation associated with `and` instruction.
    #[inline]
    fn op_and(&mut self) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let value = boolean_ops!(self, a && b);
        self.stack.push(Value::Bool(value));
        Ok(())
    }

    /// Operation associated with `or` instruction.
    #[inline]
    fn op_or(&mut self) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        let value = boolean_ops!(self, a || b);
        self.stack.push(Value::Bool(value));
        Ok(())
    }

    /// Test if the top of stack is equal to the string at the given static
    /// string location.
    #[inline]
    fn op_eq_static_string(
        &mut self,
        unit: &Rc<CompilationUnit>,
        slot: usize,
    ) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let equal = match value {
            Value::String(actual) => {
                let string = unit.lookup_string(slot)?;
                let actual = actual.get_ref()?;
                *actual == ***string
            }
            Value::StaticString(actual) => {
                let string = unit.lookup_string(slot)?;
                **actual == ***string
            }
            _ => false,
        };

        self.stack.push(Value::Bool(equal));

        Ok(())
    }

    #[inline]
    fn op_match_sequence(
        &mut self,
        ty: inst::TypeCheck,
        len: usize,
        exact: bool,
    ) -> Result<(), VmError> {
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
        unit: &Rc<CompilationUnit>,
        type_check: TypeCheck,
        slot: usize,
        exact: bool,
    ) -> Result<(), VmError> {
        let result = self.on_object_keys(unit, type_check, slot, |object, keys| {
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
    fn on_tuple<F, O>(
        &mut self,
        ty: inst::TypeCheck,
        value: &Value,
        f: F,
    ) -> Result<Option<O>, VmError>
    where
        F: FnOnce(&[Value]) -> O,
    {
        use std::slice;

        Ok(match (ty, value) {
            (inst::TypeCheck::Tuple, Value::Tuple(tuple)) => Some(f(&*tuple.get_ref()?)),
            (inst::TypeCheck::Vec, Value::Vec(vec)) => Some(f(&*vec.get_ref()?)),
            (inst::TypeCheck::Result(v), Value::Result(result)) => {
                let result = result.get_ref()?;

                Some(match (v, &*result) {
                    (inst::ResultVariant::Ok, Ok(ok)) => f(slice::from_ref(ok)),
                    (inst::ResultVariant::Err, Err(err)) => f(slice::from_ref(err)),
                    _ => return Ok(None),
                })
            }
            (inst::TypeCheck::Option(v), Value::Option(option)) => {
                let option = option.get_ref()?;

                Some(match (v, &*option) {
                    (inst::OptionVariant::Some, Some(some)) => f(slice::from_ref(some)),
                    (inst::OptionVariant::None, None) => f(&[]),
                    _ => return Ok(None),
                })
            }
            (inst::TypeCheck::Type(hash), Value::TypedTuple(typed_tuple)) => {
                let typed_tuple = typed_tuple.get_ref()?;

                if typed_tuple.hash != hash {
                    return Ok(None);
                }

                Some(f(&*typed_tuple.tuple))
            }
            (inst::TypeCheck::Variant(hash), Value::VariantTuple(variant_tuple)) => {
                let variant_tuple = variant_tuple.get_ref()?;

                if variant_tuple.hash != hash {
                    return Ok(None);
                }

                Some(f(&*variant_tuple.tuple))
            }
            (inst::TypeCheck::Unit, Value::Unit) => Some(f(&[])),
            _ => None,
        })
    }

    #[inline]
    fn on_object_keys<F, O>(
        &mut self,
        unit: &Rc<CompilationUnit>,
        type_check: TypeCheck,
        slot: usize,
        f: F,
    ) -> Result<Option<O>, VmError>
    where
        F: FnOnce(&Object<Value>, &[String]) -> O,
    {
        let value = self.stack.pop()?;

        let keys = unit
            .lookup_object_keys(slot)
            .ok_or_else(|| VmError::MissingStaticObjectKeys { slot })?;

        match (type_check, value) {
            (TypeCheck::Object, Value::Object(object)) => {
                let object = object.get_ref()?;
                return Ok(Some(f(&*object, keys)));
            }
            (TypeCheck::Type(hash), Value::TypedObject(typed_object)) => {
                let typed_object = typed_object.get_ref()?;

                if typed_object.hash == hash {
                    return Ok(Some(f(&typed_object.object, keys)));
                }
            }
            (TypeCheck::Variant(hash), Value::VariantObject(variant_object)) => {
                let variant_object = variant_object.get_ref()?;

                if variant_object.hash == hash {
                    return Ok(Some(f(&variant_object.object, keys)));
                }
            }
            _ => (),
        }

        Ok(None)
    }

    /// Construct a future from calling an async function.
    fn call_async_fn(
        &mut self,
        unit: Rc<CompilationUnit>,
        context: Rc<Context>,
        offset: usize,
        args: usize,
    ) -> Result<(), VmError> {
        let mut vm = Self::new();

        for _ in 0..args {
            vm.stack.push(self.stack.pop()?);
        }

        vm.stack.reverse();
        vm.ip = offset;

        let future = Box::leak(Box::new(async move {
            let mut task = vm.run::<Value>(unit, context);
            task.run_to_completion().await
        }));

        // Safety: future is pushed to the stack, and the stack is purged when
        // the task driving the virtual machine is dropped.
        // This ensures that the future doesn't outlive any references it uses
        // living on the stack.
        unsafe {
            let future = Future::new_unchecked(future);
            self.stack.push(Value::Future(Shared::new(future)));
        }

        Ok(())
    }

    fn call_offset_fn(
        &mut self,
        unit: &Rc<CompilationUnit>,
        context: &Rc<Context>,
        offset: usize,
        call: UnitFnCall,
        args: usize,
        update_ip: &mut bool,
    ) -> Result<(), VmError> {
        match call {
            UnitFnCall::Async => {
                self.call_async_fn(unit.clone(), context.clone(), offset, args)?;
            }
            UnitFnCall::Immediate => {
                self.push_call_frame(offset, args)?;
                *update_ip = false;
            }
        }

        Ok(())
    }

    /// Implementation of a function call.
    fn call_fn(
        &mut self,
        unit: &Rc<CompilationUnit>,
        context: &Rc<Context>,
        hash: Hash,
        args: usize,
        update_ip: &mut bool,
    ) -> Result<(), VmError> {
        match unit.lookup(hash) {
            Some(info) => {
                if info.signature.args != args {
                    return Err(VmError::ArgumentCountMismatch {
                        actual: args,
                        expected: info.signature.args,
                    });
                }

                match &info.kind {
                    UnitFnKind::Offset { offset, call } => {
                        self.call_offset_fn(unit, context, *offset, *call, args, update_ip)?;
                    }
                    UnitFnKind::Tuple { hash } => {
                        let args = info.signature.args;
                        let value = self.allocate_typed_tuple(*hash, args)?;
                        self.stack.push(value);
                    }
                    UnitFnKind::TupleVariant { enum_hash, hash } => {
                        let args = info.signature.args;
                        let value = self.allocate_tuple_variant(*enum_hash, *hash, args)?;
                        self.stack.push(value);
                    }
                }
            }
            None => {
                let handler = context
                    .lookup(hash)
                    .ok_or_else(|| VmError::MissingFunction { hash })?;

                handler(&mut self.stack, args)?;
            }
        }

        Ok(())
    }

    #[inline]
    fn op_call_instance(
        &mut self,
        unit: &Rc<CompilationUnit>,
        context: &Rc<Context>,
        hash: Hash,
        args: usize,
        update_ip: &mut bool,
    ) -> Result<(), VmError> {
        let instance = self.stack.peek()?.clone();
        let ty = instance.value_type()?;
        let hash = Hash::instance_function(ty, hash);

        match unit.lookup(hash) {
            Some(info) => {
                if info.signature.args != args {
                    return Err(VmError::ArgumentCountMismatch {
                        actual: args,
                        expected: info.signature.args,
                    });
                }

                match &info.kind {
                    UnitFnKind::Offset { offset, call } => {
                        self.call_offset_fn(unit, context, *offset, *call, args, update_ip)?;
                    }
                    UnitFnKind::Tuple { .. } => todo!("there are no instance tuple constructors"),
                    UnitFnKind::TupleVariant { .. } => {
                        todo!("there are no instance tuple constructors")
                    }
                }
            }
            None => {
                let handler = match context.lookup(hash) {
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

    fn op_call_fn(
        &mut self,
        unit: &Rc<CompilationUnit>,
        context: &Rc<Context>,
        args: usize,
        update_ip: &mut bool,
    ) -> Result<(), VmError> {
        let function = self.stack.pop()?;

        let hash = match function {
            Value::Type(hash) => hash,
            actual => {
                let actual_type = actual.type_info()?;
                return Err(VmError::UnsupportedCallFn { actual_type });
            }
        };

        self.call_fn(unit, context, hash, args, update_ip)?;
        Ok(())
    }

    /// Evaluate a single instruction.
    async fn run_for(
        &mut self,
        unit: &Rc<CompilationUnit>,
        context: &Rc<Context>,
        mut limit: Option<usize>,
    ) -> Result<(), VmError> {
        while !self.exited {
            let inst = *unit
                .instruction_at(self.ip)
                .ok_or_else(|| VmError::IpOutOfBounds)?;

            let mut update_ip = true;

            match inst {
                Inst::Not => {
                    self.op_not()?;
                }
                Inst::Add => {
                    self.op_add(context)?;
                }
                Inst::AddAssign { offset } => {
                    let arg = self.stack.pop()?;
                    let value = self.stack.at_offset(offset)?.clone();
                    let value = assign_ops! {
                        self, context, crate::ADD_ASSIGN, +, value.checked_add(arg), Overflow
                    };

                    *self.stack.at_offset_mut(offset)? = value;
                }
                Inst::Sub => {
                    self.op_sub(context)?;
                }
                Inst::SubAssign { offset } => {
                    let arg = self.stack.pop()?;
                    let value = self.stack.at_offset(offset)?.clone();
                    let value = assign_ops! {
                        self, context, crate::SUB_ASSIGN, -, value.checked_sub(arg), Underflow
                    };
                    *self.stack.at_offset_mut(offset)? = value;
                }
                Inst::Mul => {
                    self.op_mul(context)?;
                }
                Inst::MulAssign { offset } => {
                    let arg = self.stack.pop()?;
                    let value = self.stack.at_offset(offset)?.clone();
                    let value = assign_ops! {
                        self, context, crate::MUL_ASSIGN, *, value.checked_mul(arg), Overflow
                    };
                    *self.stack.at_offset_mut(offset)? = value;
                }
                Inst::Div => {
                    self.op_div(context)?;
                }
                Inst::DivAssign { offset } => {
                    let arg = self.stack.pop()?;
                    let value = self.stack.at_offset(offset)?.clone();
                    let value = assign_ops! {
                        self, context, crate::DIV_ASSIGN, /, value.checked_div(arg), DivideByZero
                    };
                    *self.stack.at_offset_mut(offset)? = value;
                }
                Inst::Call { hash, args } => {
                    self.call_fn(unit, context, hash, args, &mut update_ip)?;
                }
                Inst::CallInstance { hash, args } => {
                    self.op_call_instance(unit, context, hash, args, &mut update_ip)?;
                }
                Inst::CallFn { args } => {
                    self.op_call_fn(unit, context, args, &mut update_ip)?;
                }
                Inst::LoadInstanceFn { hash } => {
                    self.op_load_instance_fn(hash)?;
                }
                Inst::IndexGet => {
                    self.op_index_get(context)?;
                }
                Inst::VecIndexGet { index } => {
                    self.op_vec_index_get(index)?;
                }
                Inst::TupleIndexGet { index } => {
                    self.op_tuple_index_get(index)?;
                }
                Inst::ObjectSlotIndexGet { slot } => {
                    self.op_object_slot_index_get(unit, slot)?;
                }
                Inst::IndexSet => {
                    self.op_index_set(context)?;
                }
                Inst::Return => {
                    self.op_return()?;
                }
                Inst::ReturnUnit => {
                    self.op_return_unit()?;
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
                    self.op_pop_and_jump_if(count, offset, &mut update_ip)?;
                }
                Inst::PopAndJumpIfNot { count, offset } => {
                    self.op_pop_and_jump_if_not(count, offset, &mut update_ip)?;
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
                    self.op_jump(offset, &mut update_ip)?;
                }
                Inst::JumpIf { offset } => {
                    self.op_jump_if(offset, &mut update_ip)?;
                }
                Inst::JumpIfNot { offset } => {
                    self.op_jump_if_not(offset, &mut update_ip)?;
                }
                Inst::JumpIfBranch { branch, offset } => {
                    self.op_jump_if_branch(branch, offset, &mut update_ip)?;
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
                    self.op_object(unit, slot)?;
                }
                Inst::TypedObject { hash, slot } => {
                    self.op_typed_object(unit, hash, slot)?;
                }
                Inst::VariantObject {
                    enum_hash,
                    hash,
                    slot,
                } => {
                    self.op_variant_object(unit, enum_hash, hash, slot)?;
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
                    self.op_string(unit, slot)?;
                }
                Inst::Bytes { slot } => {
                    let bytes = unit.lookup_bytes(slot)?.to_owned();
                    // TODO: do something sneaky to only allocate the static byte string once.
                    let value = Value::Bytes(Shared::new(Bytes::from_vec(bytes)));
                    self.stack.push(value);
                }
                Inst::StringConcat { len, size_hint } => {
                    self.op_string_concat(context, len, size_hint)?;
                }
                Inst::Is => {
                    self.op_is(unit, context)?;
                }
                Inst::IsNot => {
                    self.op_is_not(unit, context)?;
                }
                Inst::IsUnit => {
                    let value = self.stack.pop()?;
                    self.stack.push(Value::Bool(matches!(value, Value::Unit)));
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
                    let value = self.stack.pop()?;

                    self.stack.push(Value::Bool(match value {
                        Value::Byte(actual) => actual == byte,
                        _ => false,
                    }));
                }
                Inst::EqCharacter { character } => {
                    let value = self.stack.pop()?;

                    self.stack.push(Value::Bool(match value {
                        Value::Char(actual) => actual == character,
                        _ => false,
                    }));
                }
                Inst::EqInteger { integer } => {
                    let value = self.stack.pop()?;

                    self.stack.push(Value::Bool(match value {
                        Value::Integer(actual) => actual == integer,
                        _ => false,
                    }));
                }
                Inst::EqStaticString { slot } => {
                    self.op_eq_static_string(unit, slot)?;
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
                    self.op_match_object(unit, type_check, slot, exact)?;
                }
                Inst::Panic { reason } => {
                    return Err(VmError::Panic {
                        reason: Panic::from(reason),
                    });
                }
            }

            if update_ip {
                self.ip += 1;
            }

            if let Some(limit) = &mut limit {
                if *limit <= 1 {
                    break;
                }

                *limit -= 1;
            }
        }

        Ok(())
    }
}

/// The task of a unit being run.
pub struct Task<'a, T> {
    /// The virtual machine associated with the task.
    vm: &'a mut Vm,
    /// The compilation unit.
    unit: Rc<CompilationUnit>,
    /// Functions collection associated with the task.
    context: Rc<Context>,
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
    pub fn unit(&self) -> &CompilationUnit {
        &*self.unit
    }

    /// Run the given task to completion.
    pub async fn run_to_completion(&mut self) -> Result<T, VmError> {
        while !self.vm.exited {
            match self.vm.run_for(&self.unit, &self.context, None).await {
                Ok(()) => (),
                Err(e) => return Err(e),
            }
        }

        let value = self.vm.pop_decode()?;
        debug_assert!(self.vm.stack.is_empty());
        Ok(value)
    }

    /// Step the given task until the return value is available.
    pub async fn step(&mut self) -> Result<Option<T>, VmError> {
        self.vm.run_for(&self.unit, &self.context, Some(1)).await?;

        if self.vm.exited {
            let value = self.vm.pop_decode()?;
            debug_assert!(self.vm.stack.is_empty());
            return Ok(Some(value));
        }

        Ok(None)
    }
}

impl<T> Drop for Task<'_, T> {
    fn drop(&mut self) {
        // NB: this is critical for safety, since the stack might contain
        // references passed in externally which are tied to our lifetime ('a).
        self.vm.clear();
    }
}
