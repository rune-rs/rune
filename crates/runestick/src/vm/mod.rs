use crate::any::Any;
use crate::collections::HashMap;
use crate::context::Context;
use crate::future::{Future, SelectFuture};
use crate::hash::Hash;
use crate::reflection::{FromValue, IntoArgs};
use crate::tls;
use crate::unit::CompilationUnit;
use crate::value::{Slot, Value, ValuePtr, ValueRef, ValueTypeInfo};
use std::any;
use std::cell::UnsafeCell;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::sync::Arc;
use thiserror::Error;

mod access;
mod inst;
mod slots;

use self::access::Access;
pub use self::access::{Mut, RawMutGuard, RawRefGuard, Ref};
pub use self::inst::{Inst, Panic};
use self::slots::Slots;

/// A type-erased rust number.
#[derive(Debug, Clone, Copy)]
pub enum Integer {
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

impl fmt::Display for Integer {
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

/// Errors raised by the execution of the virtual machine.
#[derive(Debug, Error)]
pub enum VmError {
    /// The virtual machine panicked for a specific reason.
    #[error("panicked `{reason}`")]
    Panic {
        /// The reason for the panic.
        reason: Panic,
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
    /// Error raised in a user-defined function.
    #[error("error in user-defined function: {error}")]
    UserError {
        /// Source error.
        #[from]
        error: crate::error::Error,
    },
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
    /// Failure to lookup module.
    #[error("missing module with hash `{module}`")]
    MissingModule {
        /// Hash of module to look up.
        module: Hash,
    },
    /// Failure to lookup function in a module.
    #[error("missing function with hash `{hash}` in module with hash `{module}`")]
    MissingModuleFunction {
        /// Module that was looked up.
        module: Hash,
        /// Function that could not be found.
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
    /// Unsupported argument to object-exact-keys.
    #[error("unsupported object key `{actual}`")]
    UnsupportedObjectKey {
        /// The encountered argument.
        actual: ValueTypeInfo,
    },
    /// Unsupported argument to string-concat
    #[error("unsupported string-concat argument `{actual}`")]
    UnsupportedStringConcatArgument {
        /// The encountered argument.
        actual: ValueTypeInfo,
    },
    /// Attempt to access out-of-bounds stack item.
    #[error("tried to access an out-of-bounds stack entry")]
    StackOutOfBounds,
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
    /// Saw an unexpected stack value.
    #[error("unexpected stack value, expected `{expected}` but was `{actual}`")]
    StackTopTypeError {
        /// The type that was expected.
        expected: ValueTypeInfo,
        /// The type observed.
        actual: ValueTypeInfo,
    },
    /// Indicates a failure to convert from one type to another.
    #[error("failed to convert stack value from `{from}` to `{to}`")]
    StackConversionError {
        /// The source of the error.
        #[source]
        error: Box<VmError>,
        /// The actual type to be converted.
        from: ValueTypeInfo,
        /// The expected type to convert towards.
        to: &'static str,
    },
    /// Failure to convert from one type to another.
    #[error("failed to convert argument #{arg} from `{from}` to `{to}`")]
    ArgumentConversionError {
        /// The underlying stack error.
        #[source]
        error: Box<VmError>,
        /// The argument location that was converted.
        arg: usize,
        /// The value type we attempted to convert from.
        from: ValueTypeInfo,
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
        error: Box<VmError>,
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
    /// An array index get operation that is not supported.
    #[error("the array index get operation is not supported on `{target_type}`")]
    UnsupportedArrayIndexGet {
        /// The target type we tried to perform the array indexing on.
        target_type: ValueTypeInfo,
    },
    /// An array index get operation that is not supported.
    #[error("the tuple index get operation is not supported on `{target_type}`")]
    UnsupportedTupleIndexGet {
        /// The target type we tried to perform the array indexing on.
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
    /// Attempting to assign an illegal pointer.
    #[error(
        "pointer cannot be changed to point to a lower stack address `{value_ptr} > {target_ptr}`"
    )]
    IllegalPtrReplace {
        /// The target ptr being assigned to.
        target_ptr: usize,
        /// The value ptr we are trying to assign.
        value_ptr: usize,
    },
    /// Encountered a value that could not be called as a function
    #[error("`{actual_type}` cannot be called since it's not a function")]
    UnsupportedCallFn {
        /// The type that could not be called.
        actual_type: ValueTypeInfo,
    },
    /// Tried to fetch an index in an array that doesn't exist.
    #[error("missing index `{index}` in array")]
    ArrayIndexMissing {
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

    /// Internal error that happens when we run out of items in a list.
    #[error("unexpectedly ran out of items to iterate over")]
    IterationError,
    /// stack is empty
    #[error("stack is empty")]
    StackEmpty,
    /// Attempt to pop outside of current frame offset.
    #[error("attempted to pop beyond current stack frame `{frame}`")]
    PopOutOfBounds {
        /// CallFrame offset that we tried to pop.
        frame: usize,
    },
    /// The given slot is missing.
    #[error("missing slot `{slot}`")]
    SlotMissing {
        /// The slot that was missing.
        slot: Slot,
    },
    /// The given slot is inaccessible.
    #[error("`{slot}` is inaccessible for exclusive access")]
    SlotInaccessibleExclusive {
        /// The slot that could not be accessed.
        slot: Slot,
    },
    /// The given slot is inaccessible.
    #[error("`{slot}` is inaccessible for shared access")]
    SlotInaccessibleShared {
        /// The slot that could not be accessed.
        slot: Slot,
    },
    /// Error raised when we expect a specific external type but got another.
    #[error("expected slot `{expected}`, but found `{actual}`")]
    UnexpectedSlotType {
        /// The type that was expected.
        expected: &'static str,
        /// The type that was found.
        actual: &'static str,
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
    /// Error raised when we fail to unwrap an option.
    #[error("expected some value, but found none")]
    ExpectedOptionSome,
    /// Error raised when we expecting a result.
    #[error("expected result, but found `{actual}`")]
    ExpectedResult {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expecting an ok result.
    #[error("expected ok result, but found error `{error}`")]
    ExpectedResultOk {
        /// The error found.
        error: ValueTypeInfo,
    },
    /// Error raised when we expected a boolean value.
    #[error("expected booleant, but found `{actual}`")]
    ExpectedBoolean {
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
    /// Error raised when we expected a array.
    #[error("expected a array but found `{actual}`")]
    ExpectedArray {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected an array of the given length.
    #[error("expected a array of length `{expected}`, but found one with length `{actual}`")]
    ExpectedArrayLength {
        /// The actual length observed.
        actual: usize,
        /// The expected array length.
        expected: usize,
    },
    /// Error raised when we expected a object.
    #[error("expected a object but found `{actual}`")]
    ExpectedObject {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected an external value.
    #[error("expected a external value but found `{actual}`")]
    ExpectedExternal {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a managed value.
    #[error("expected an external, array, object, or string, but found `{actual}`")]
    ExpectedManaged {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a future.
    #[error("expected future, but found `{actual}`")]
    ExpectedFuture {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a managed value with a specific slot.
    #[error("slot type is incompatible with expected")]
    IncompatibleSlot,
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
    /// We encountered a corrupted stack frame.
    #[error("stack size `{stack_top}` starts before the current stack frame `{frame_at}`")]
    CorruptedStackFrame {
        /// The size of the stack.
        stack_top: usize,
        /// The location of the stack frame.
        frame_at: usize,
    },
    /// Error raised when the branch register is empty.
    #[error("branch register empty")]
    BranchEmpty {},
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

/// Pop and type check a value off the stack.
macro_rules! pop {
    ($vm:expr, $variant:ident) => {
        match $vm.pop()? {
            ValuePtr::$variant(b) => b,
            other => {
                return Err(VmError::StackTopTypeError {
                    expected: ValueTypeInfo::$variant,
                    actual: other.type_info($vm)?,
                })
            }
        }
    };
}

/// Generate a primitive combination of operations.
macro_rules! primitive_ops {
    ($vm:expr, $a:ident $op:tt $b:ident) => {
        match ($a, $b) {
            (ValuePtr::Char($a), ValuePtr::Char($b)) => $a $op $b,
            (ValuePtr::Bool($a), ValuePtr::Bool($b)) => $a $op $b,
            (ValuePtr::Integer($a), ValuePtr::Integer($b)) => $a $op $b,
            (ValuePtr::Float($a), ValuePtr::Float($b)) => $a $op $b,
            (lhs, rhs) => return Err(VmError::UnsupportedBinaryOperation {
                op: stringify!($op),
                lhs: lhs.type_info($vm)?,
                rhs: rhs.type_info($vm)?,
            }),
        }
    }
}

/// Generate a boolean combination of operations.
macro_rules! boolean_ops {
    ($vm:expr, $a:ident $op:tt $b:ident) => {
        match ($a, $b) {
            (ValuePtr::Bool($a), ValuePtr::Bool($b)) => $a $op $b,
            (lhs, rhs) => return Err(VmError::UnsupportedBinaryOperation {
                op: stringify!($op),
                lhs: lhs.type_info($vm)?,
                rhs: rhs.type_info($vm)?,
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
            (ValuePtr::Integer($a), ValuePtr::Integer($b)) => {
                $vm.push(ValuePtr::Integer({
                    match $a.$checked_op($b) {
                        Some(value) => value,
                        None => return Err(VmError::$error),
                    }
                }));
            },
            (ValuePtr::Float($a), ValuePtr::Float($b)) => {
                $vm.push(ValuePtr::Float(check_float!($a $op $b, $error)));
            },
            (lhs, rhs) => {
                let ty = lhs.value_type($vm)?;
                let hash = Hash::instance_function(ty, *$fn);

                let handler = match $context.lookup(hash) {
                    Some(handler) => handler,
                    None => {
                        return Err(VmError::UnsupportedBinaryOperation {
                            op: stringify!($op),
                            lhs: lhs.type_info($vm)?,
                            rhs: rhs.type_info($vm)?,
                        });
                    }
                };

                $vm.push(rhs);
                $vm.push(lhs);
                handler($vm, 1)?;
            },
        }
    }
}

/// Generate a primitive combination of operations.
macro_rules! assign_ops {
    ($vm:expr, $context:expr, $fn:expr, $op:tt, $a:ident . $checked_op:ident ( $b:ident ), $error:ident) => {
        match ($a, $b) {
            (ValuePtr::Integer($a), ValuePtr::Integer($b)) => ValuePtr::Integer({
                match $a.$checked_op($b) {
                    Some(value) => value,
                    None => return Err(VmError::$error),
                }
            }),
            (ValuePtr::Float($a), ValuePtr::Float($b)) => ValuePtr::Float({
                check_float!($a $op $b, $error)
            }),
            (lhs, rhs) => {
                let ty = lhs.value_type($vm)?;
                let hash = Hash::instance_function(ty, *$fn);

                let handler = match $context.lookup(hash) {
                    Some(handler) => handler,
                    None => {
                        return Err(VmError::UnsupportedBinaryOperation {
                            op: stringify!($op),
                            lhs: lhs.type_info($vm)?,
                            rhs: rhs.type_info($vm)?,
                        });
                    }
                };

                $vm.push(rhs);
                $vm.push(lhs);
                handler($vm, 1)?;
                $vm.pop()?;
                lhs
            }
        }
    }
}

/// The holder of an external value.
///
/// This behaves a lot like RefCell, but with some extra tricks up its sleeve,
/// like the ability to construct raw access guards. See [Ref] and [Mut].
#[derive(Debug)]
struct Holder {
    /// The generation this holder was created for.
    ///
    /// Slots referencing this holder encoder the generation, so a slot will
    /// become invalid if the generation between the holder and the slot does
    /// not match.
    generation: usize,
    /// How the external is accessed (if it is accessed).
    /// This only happens during function calls, and the function callee is
    /// responsible for unwinding the access.
    ///
    /// Note that this is allocated on the heap, because guards referencing it
    /// will use (unsafe) pointers to it, and we can't have it move.
    /// Other safety guarantees of the virtual machinea asserts that the holder
    /// is not deallocates as well.
    access: Box<Access>,
    /// The value being held. Guarded by the `access` field to determine if it
    /// can be access shared or exclusively.
    value: UnsafeCell<Any>,
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

macro_rules! call_fn {
    ($vm:expr, $hash:expr, $args:expr, $context:expr,  $update_ip:ident) => {
        let hash = $hash;

        match $vm.unit.lookup_offset(hash) {
            Some(loc) => {
                $vm.push_call_frame(loc, $args)?;
                $update_ip = false;
            }
            None => {
                let handler = $context
                    .lookup(hash)
                    .ok_or_else(|| VmError::MissingFunction { hash })?;

                handler($vm, $args)?;
            }
        }
    };
}

/// Implements slot functions for a given type.
///
/// This provides typed convenience functions over the typical:
/// * [slot_allocate]
/// * [external_ref]
/// * [external_mut]
/// * [external_take]
/// * [external_clone]
macro_rules! impl_slot_functions {
    (
        $ty:ty,
        $slot:ident,
        $allocate_fn:ident,
        $ref_fn:ident,
        $mut_fn:ident,
        $take_fn:ident,
        $($clone_fn:ident,)?
    ) => {
        /// Allocate a value and return its ptr.
        pub fn $allocate_fn(&mut self, value: $ty) -> ValuePtr {
            ValuePtr::$slot(self.slot_allocate(value))
        }

        /// Get a reference of the value at the given slot.
        pub fn $ref_fn(&self, slot: Slot) -> Result<Ref<'_, $ty>, VmError> {
            self.external_ref::<$ty>(slot)
        }

        /// Get a reference of the value at the given slot.
        pub fn $mut_fn(&self, slot: Slot) -> Result<Mut<'_, $ty>, VmError> {
            self.external_mut::<$ty>(slot)
        }

        /// Take the value at the given slot.
        pub fn $take_fn(&mut self, slot: Slot) -> Result<$ty, VmError> {
            self.external_take::<$ty>(slot)
        }

        $(
            /// Get a cloned value from the given slot.
            pub fn $clone_fn(&self, slot: Slot) -> Result<$ty, VmError> {
                self.external_clone::<$ty>(slot)
            }
        )*
    };
}

#[derive(Debug)]
pub struct Stack {
    /// The current stack of values.
    stack: Vec<ValuePtr>,
    /// The top of the current stack frame.
    stack_top: usize,
}

impl Stack {
    /// Construct a new stack.
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            stack_top: 0,
        }
    }

    /// Clear the current stack.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.stack_top = 0;
    }

    /// Peek the top of the stack.
    fn peek(&mut self) -> Result<ValuePtr, VmError> {
        self.stack
            .last()
            .copied()
            .ok_or_else(|| VmError::StackEmpty)
    }

    /// Get the last position on the stack.
    pub fn last(&self) -> Result<ValuePtr, VmError> {
        self.stack
            .last()
            .copied()
            .ok_or_else(|| VmError::StackEmpty)
    }

    /// Access the value at the given frame offset.
    fn at_offset(&self, offset: usize) -> Result<ValuePtr, VmError> {
        self.stack_top
            .checked_add(offset)
            .and_then(|n| self.stack.get(n).copied())
            .ok_or_else(|| VmError::StackOutOfBounds)
    }

    /// Get the offset at the given location.
    fn at_offset_mut(&mut self, offset: usize) -> Result<&mut ValuePtr, VmError> {
        let n = match self.stack_top.checked_add(offset) {
            Some(n) => n,
            None => return Err(VmError::StackOutOfBounds),
        };

        match self.stack.get_mut(n) {
            Some(value) => Ok(value),
            None => Err(VmError::StackOutOfBounds),
        }
    }

    /// Push an unmanaged reference.
    ///
    /// The reference count of the value being referenced won't be modified.
    pub fn push(&mut self, value: ValuePtr) {
        self.stack.push(value);
    }

    /// Pop a reference to a value from the stack.
    pub fn pop(&mut self) -> Result<ValuePtr, VmError> {
        if self.stack.len() == self.stack_top {
            return Err(VmError::PopOutOfBounds {
                frame: self.stack_top,
            });
        }

        self.stack.pop().ok_or_else(|| VmError::StackEmpty)
    }

    /// Get the length of the stack.
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Iterate over the stack.
    pub fn iter(&self) -> impl Iterator<Item = ValuePtr> + '_ {
        self.stack.iter().copied()
    }
}

/// A stack which references variables indirectly from a slab.
pub struct Vm {
    /// The current instruction pointer.
    ip: usize,
    /// The current stack.
    stack: Stack,
    /// We have exited from the last frame.
    exited: bool,
    /// Frames relative to the stack.
    call_frames: Vec<CallFrame>,
    /// Slots with external values.
    slots: Slots,
    /// Generation used for allocated objects.
    generation: usize,
    /// The compilation unit associated with the virtual machine.
    pub(crate) unit: Arc<CompilationUnit>,
    /// The `branch` registry used for certain operations.
    branch: Option<usize>,
}

impl Vm {
    /// Construct a new runestick virtual machine.
    pub fn new(unit: Arc<CompilationUnit>) -> Self {
        Self {
            ip: 0,
            stack: Stack::new(),
            exited: false,
            call_frames: Vec::new(),
            slots: Slots::new(),
            generation: 0,
            unit,
            branch: None,
        }
    }

    /// Access the underlying compilation unit.
    pub fn unit(&self) -> &CompilationUnit {
        &*self.unit
    }

    /// Reset this virtual machine, freeing all memory used.
    pub fn clear(&mut self) {
        self.ip = 0;
        self.exited = false;
        self.stack.clear();
        self.call_frames.clear();
        self.slots.clear();
        self.generation = 0;
    }

    /// Push an unmanaged reference.
    ///
    /// The reference count of the value being referenced won't be modified.
    pub fn push(&mut self, value: ValuePtr) {
        self.stack.push(value);
    }

    /// Pop a reference to a value from the stack.
    pub fn pop(&mut self) -> Result<ValuePtr, VmError> {
        self.stack.pop()
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
    pub fn iter_stack_debug(
        &self,
    ) -> impl Iterator<Item = (ValuePtr, Result<ValueRef<'_>, VmError>)> + '_ {
        let mut it = self.stack.iter();

        std::iter::from_fn(move || {
            let value_ref = it.next()?;
            let value = self.value_ref(value_ref);
            Some((value_ref, value))
        })
    }

    /// Call the given function in the given compilation unit.
    pub fn call_function<'a, A: 'a, T, I>(
        mut self,
        context: Arc<Context>,
        name: I,
        args: A,
    ) -> Result<Task<'a, T>, VmError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
        A: 'a + IntoArgs,
        T: FromValue,
    {
        let hash = Hash::function(name);

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

        self.ip = function.offset;
        self.stack.clear();

        // Safety: we bind the lifetime of the arguments to the outgoing task,
        // ensuring that the task won't outlive any potentially passed in
        // references.
        unsafe {
            args.into_args(&mut self)?;
        }

        Ok(Task {
            vm: self,
            context,
            _marker: PhantomData,
        })
    }

    /// Run the given program on the virtual machine.
    pub fn run<'a, T>(self, context: Arc<Context>) -> Task<'a, T>
    where
        T: FromValue,
    {
        Task {
            vm: self,
            context,
            _marker: PhantomData,
        }
    }

    async fn op_await(&mut self) -> Result<(), VmError> {
        let value = self.pop()?;

        let future = match value {
            ValuePtr::Future(slot) => self.future_take(slot)?,
            actual => {
                return Err(VmError::ExpectedFuture {
                    actual: actual.type_info(self)?,
                })
            }
        };

        unsafe {
            tls::InjectVm::new(self, future).await?;
        }

        Ok(())
    }

    async fn op_select(&mut self, len: usize) -> Result<(), VmError> {
        use futures::stream::StreamExt as _;

        let branch = {
            let mut futures = futures::stream::FuturesUnordered::new();
            let mut guards = Vec::new();

            for index in 0..len {
                let value = self.stack.pop()?;

                let future = match value {
                    ValuePtr::Future(slot) => self.future_mut(slot)?,
                    actual => {
                        return Err(VmError::ExpectedFuture {
                            actual: actual.type_info(self)?,
                        })
                    }
                };

                if future.is_completed() {
                    continue;
                }

                // Safety: we have exclusive access to the virtual machine, so we
                // can assert that nothing is invalidate for the duration of this
                // select.
                unsafe {
                    let (raw_future, guard) = Mut::unsafe_into_mut(future);
                    futures.push(SelectFuture::new_unchecked(raw_future, index));
                    guards.push(guard);
                };
            }

            // NB: nothing to poll.
            if futures.is_empty() {
                return Ok(());
            }

            let result = unsafe { tls::InjectVm::new(self, futures.next()).await.unwrap() };
            let index = result?;
            drop(guards);
            index
        };

        self.branch = Some(branch);
        Ok(())
    }

    /// Pop a number of values from the stack.
    fn op_popn(&mut self, n: usize) -> Result<(), VmError> {
        if self.stack.len().saturating_sub(self.stack.stack_top) < n {
            return Err(VmError::PopOutOfBounds {
                frame: self.stack.stack_top,
            });
        }

        for _ in 0..n {
            self.stack.pop()?;
        }

        Ok(())
    }

    /// Pop a number of values from the stack, while preserving the top of the
    /// stack.
    fn op_clean(&mut self, n: usize) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        self.op_popn(n)?;
        self.push(value);
        Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    fn do_copy(&mut self, offset: usize) -> Result<(), VmError> {
        let value = self.stack.at_offset(offset)?;
        self.stack.push(value);
        Ok(())
    }

    /// Duplicate the value at the top of the stack.
    fn do_dup(&mut self) -> Result<(), VmError> {
        let value = self.stack.last()?;
        self.stack.push(value);
        Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    fn do_replace(&mut self, offset: usize) -> Result<(), VmError> {
        let mut value = self.stack.pop()?;
        let stack_value = self.stack.at_offset_mut(offset)?;
        mem::swap(stack_value, &mut value);
        Ok(())
    }

    /// Push a new call frame.
    fn push_call_frame(&mut self, new_ip: usize, args: usize) -> Result<(), VmError> {
        let offset = self
            .stack
            .len()
            .checked_sub(args)
            .ok_or_else(|| VmError::StackOutOfBounds)?;

        self.call_frames.push(CallFrame {
            ip: self.ip,
            stack_top: self.stack.stack_top,
        });

        self.stack.stack_top = offset;
        self.ip = new_ip;
        Ok(())
    }

    /// Pop a call frame and return it.
    fn pop_call_frame(&mut self) -> Result<bool, VmError> {
        // Assert that the stack frame has been restored to the previous top
        // at the point of return.
        if self.stack.len() != self.stack.stack_top {
            return Err(VmError::CorruptedStackFrame {
                stack_top: self.stack.len(),
                frame_at: self.stack.stack_top,
            });
        }

        let frame = match self.call_frames.pop() {
            Some(frame) => frame,
            None => return Ok(true),
        };

        self.stack.stack_top = frame.stack_top;
        self.ip = frame.ip;
        Ok(false)
    }

    fn internal_allocate<T>(&mut self, generation: usize, value: T) -> usize
    where
        T: any::Any,
    {
        self.slots.insert(Holder {
            generation,
            access: Box::new(Access::default()),
            value: UnsafeCell::new(Any::new(value)),
        })
    }

    fn generation(&mut self) -> usize {
        let g = self.generation;
        self.generation += 1;
        g
    }

    /// Allocate and insert an external and return its reference.
    ///
    /// This will leak memory unless the reference is pushed onto the stack to
    /// be managed.
    pub fn slot_allocate<T>(&mut self, value: T) -> Slot
    where
        T: any::Any,
    {
        let generation = self.generation();
        Slot::new(generation, self.internal_allocate(generation, value))
    }

    /// Allocate and insert an external and return its reference.
    ///
    /// This will leak memory unless the reference is pushed onto the stack to
    /// be managed.
    pub fn external_allocate<T>(&mut self, value: T) -> ValuePtr
    where
        T: any::Any,
    {
        ValuePtr::External(self.slot_allocate(value))
    }

    /// Allocate an external slot for the given reference.
    ///
    /// # Safety
    ///
    /// If the pointer was constructed from a reference, the reference passed in
    /// MUST NOT be used again until the VM has been cleared using
    /// [clear][Self::clear] since the VM is actively aliasing the reference for
    /// the duration of its life.
    pub unsafe fn external_allocate_ptr<T>(&mut self, value: *const T) -> ValuePtr
    where
        T: any::Any,
    {
        let generation = self.generation();

        let any = Any::from_ptr(value);

        let index = self.slots.insert(Holder {
            generation,
            access: Box::new(Access::default()),
            value: UnsafeCell::new(any),
        });

        ValuePtr::External(Slot::new(generation, index))
    }

    /// Allocate an external slot for the given mutable reference.
    ///
    /// # Safety
    ///
    /// If the pointer was constructed from a reference, the reference passed in
    /// MUST NOT be used again until the VM has been cleared using
    /// [clear][Self::clear] since the VM is actively aliasing the reference for
    /// the duration of its life.
    pub unsafe fn external_allocate_mut_ptr<T>(&mut self, value: *mut T) -> ValuePtr
    where
        T: any::Any,
    {
        let generation = self.generation();

        let any = Any::from_mut_ptr(value);

        let index = self.slots.insert(Holder {
            generation,
            access: Box::new(Access::default()),
            value: UnsafeCell::new(any),
        });

        ValuePtr::External(Slot::new(generation, index))
    }

    impl_slot_functions! {
        String,
        String,
        string_allocate,
        string_ref,
        string_mut,
        string_take,
        string_clone,
    }

    impl_slot_functions! {
        Vec<ValuePtr>,
        Array,
        array_allocate,
        array_ref,
        array_mut,
        array_take,
        array_clone,
    }

    impl_slot_functions! {
        HashMap<String, ValuePtr>,
        Object,
        object_allocate,
        object_ref,
        object_mut,
        object_take,
        object_clone,
    }

    impl_slot_functions! {
        Future,
        Future,
        future_allocate,
        future_ref,
        future_mut,
        future_take,
    }

    impl_slot_functions! {
        Result<ValuePtr, ValuePtr>,
        Result,
        result_allocate,
        result_ref,
        result_mut,
        result_take,
        result_clone,
    }

    impl_slot_functions! {
        Option<ValuePtr>,
        Option,
        option_allocate,
        option_ref,
        option_mut,
        option_take,
        option_clone,
    }

    impl_slot_functions! {
        Box<[ValuePtr]>,
        Tuple,
        tuple_allocate,
        tuple_ref,
        tuple_mut,
        tuple_take,
        tuple_clone,
    }

    /// Get a reference of the external value of the given type and the given
    /// slot.
    pub fn external_ref<T>(&self, slot: Slot) -> Result<Ref<'_, T>, VmError>
    where
        T: any::Any,
    {
        let holder = self
            .slots
            .get(slot.into_usize(), slot.into_generation())
            .ok_or_else(|| VmError::SlotMissing { slot })?;

        holder.access.shared(slot)?;

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        unsafe {
            let value = match (*holder.value.get()).as_ptr(any::TypeId::of::<T>()) {
                Some(value) => value,
                None => {
                    let actual = (*holder.value.get()).type_name();

                    // NB: Immediately unshare because the cast failed and we
                    // won't be maintaining access to the type.
                    holder.access.release_shared();

                    return Err(VmError::UnexpectedSlotType {
                        expected: any::type_name::<T>(),
                        actual,
                    });
                }
            };

            Ok(Ref {
                value: &*(value as *const T),
                raw: RawRefGuard {
                    access: &*holder.access,
                },
            })
        }
    }

    /// Get a mutable reference of the external value of the given type and the
    /// given slot.
    ///
    /// Mark the given value as mutably used, preventing it from being used
    /// again.
    pub fn external_mut<T>(&self, slot: Slot) -> Result<Mut<'_, T>, VmError>
    where
        T: any::Any,
    {
        let holder = self
            .slots
            .get(slot.into_usize(), slot.into_generation())
            .ok_or_else(|| VmError::SlotMissing { slot })?;

        holder.access.exclusive(slot)?;

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        unsafe {
            let value = match (*holder.value.get()).as_mut_ptr(any::TypeId::of::<T>()) {
                Some(value) => value,
                None => {
                    let actual = (*holder.value.get()).type_name();

                    // NB: Immediately unshare because the cast failed and we
                    // won't be maintaining access to the type.
                    holder.access.release_exclusive();

                    return Err(VmError::UnexpectedSlotType {
                        expected: any::type_name::<T>(),
                        actual,
                    });
                }
            };

            Ok(Mut {
                value: &mut *(value as *mut T),
                raw: RawMutGuard {
                    access: &*holder.access,
                },
            })
        }
    }

    /// Get a clone of the given external.
    pub fn external_clone<T: Clone + any::Any>(&self, slot: Slot) -> Result<T, VmError> {
        let holder = self
            .slots
            .get(slot.into_usize(), slot.into_generation())
            .ok_or_else(|| VmError::SlotMissing { slot })?;

        // NB: we don't need a guard here since we're only using the reference
        // for the duration of this function.
        holder.access.test_shared(slot)?;

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        unsafe {
            let value = match (*holder.value.get()).as_ptr(any::TypeId::of::<T>()) {
                Some(value) => &*(value as *const T),
                None => {
                    let actual = (*holder.value.get()).type_name();

                    return Err(VmError::UnexpectedSlotType {
                        expected: any::type_name::<T>(),
                        actual,
                    });
                }
            };

            Ok(value.clone())
        }
    }

    /// Try to convert the value.
    ///
    /// Returns the value which we couldn't convert in case it cannot be converted.
    fn take_value<T>(value: Any) -> Result<T, Any>
    where
        T: 'static,
    {
        // Safety: The conversion is fully checked through the invariants
        // provided by our custom `Any` implementaiton.
        //
        // `as_mut_ptr` ensures that the type of the boxed value matches the
        // expected type.
        unsafe {
            match value.take_mut_ptr(any::TypeId::of::<T>()) {
                Ok(ptr) => Ok(*Box::from_raw(ptr as *mut T)),
                Err(any) => Err(any),
            }
        }
    }

    /// Take an external value from the virtual machine by its slot.
    pub fn external_take<T>(&mut self, slot: Slot) -> Result<T, VmError>
    where
        T: any::Any,
    {
        // NB: don't need to perform a runtime check because this function
        // requires exclusive access to the virtual machine, at which point it's
        // impossible for live references to slots to be out unless unsafe
        // functions have been used in an unsound manner.
        let holder = match self.slots.remove(slot.into_usize(), slot.into_generation()) {
            Some(holder) => holder,
            None => {
                return Err(VmError::SlotMissing { slot });
            }
        };

        match Self::take_value(holder.value.into_inner()) {
            Ok(value) => Ok(value),
            Err(value) => Err(VmError::UnexpectedSlotType {
                expected: any::type_name::<T>(),
                actual: value.type_name(),
            }),
        }
    }

    fn external_with_dyn<F, T>(&self, slot: Slot, f: F) -> Result<T, VmError>
    where
        F: FnOnce(&Any) -> T,
    {
        let holder = self
            .slots
            .get(slot.into_usize(), slot.into_generation())
            .ok_or_else(|| VmError::SlotMissing { slot })?;

        holder.access.test_shared(slot)?;

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        Ok(f(unsafe { &*holder.value.get() }))
    }

    /// Get a reference of the external value of the given type and the given
    /// slot.
    pub fn external_ref_dyn(&self, slot: Slot) -> Result<Ref<'_, Any>, VmError> {
        let holder = self
            .slots
            .get(slot.into_usize(), slot.into_generation())
            .ok_or_else(|| VmError::SlotMissing { slot })?;

        holder.access.shared(slot)?;

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        Ok(Ref {
            value: unsafe { &*holder.value.get() },
            raw: RawRefGuard {
                access: &*holder.access,
            },
        })
    }

    /// Take an external value by dyn, assuming you have exlusive access to it.
    pub fn external_take_dyn(&mut self, slot: Slot) -> Result<Any, VmError> {
        let holder = match self.slots.remove(slot.into_usize(), slot.into_generation()) {
            Some(holder) => holder,
            None => {
                return Err(VmError::SlotMissing { slot });
            }
        };

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        Ok(holder.value.into_inner())
    }

    /// Access the type name of the slot.
    pub fn slot_type_name(&self, slot: Slot) -> Result<&'static str, VmError> {
        self.external_with_dyn(slot, |e| e.type_name())
    }

    /// Access the type id of the slot.
    pub fn slot_type_id(&self, slot: Slot) -> Result<any::TypeId, VmError> {
        self.external_with_dyn(slot, |e| e.type_id())
    }

    /// Convert a value reference into an owned value.
    pub fn value_take(&mut self, value: ValuePtr) -> Result<Value, VmError> {
        return Ok(match value {
            ValuePtr::Unit => Value::Unit,
            ValuePtr::Integer(integer) => Value::Integer(integer),
            ValuePtr::Float(float) => Value::Float(float),
            ValuePtr::Bool(boolean) => Value::Bool(boolean),
            ValuePtr::Char(c) => Value::Char(c),
            ValuePtr::String(slot) => Value::String(self.string_take(slot)?),
            ValuePtr::StaticString(slot) => {
                Value::String(self.unit.lookup_string(slot)?.to_owned())
            }
            ValuePtr::Array(slot) => {
                let array = self.array_take(slot)?;
                Value::Array(value_take_array(self, array)?)
            }
            ValuePtr::Tuple(slot) => {
                let tuple = self.tuple_take(slot)?;
                Value::Tuple(value_take_tuple(self, &*tuple)?)
            }
            ValuePtr::Object(slot) => {
                let object = self.object_take(slot)?;
                Value::Object(value_take_object(self, object)?)
            }
            ValuePtr::External(slot) => Value::External(self.external_take_dyn(slot)?),
            ValuePtr::Type(ty) => Value::Type(ty),
            ValuePtr::Fn(hash) => Value::Fn(hash),
            ValuePtr::Future(slot) => {
                let future = self.external_take(slot)?;
                Value::Future(future)
            }
            ValuePtr::Option(slot) => {
                let option = self.external_take(slot)?;

                let option = match option {
                    Some(slot) => Some(Box::new(self.value_take(slot)?)),
                    None => None,
                };

                Value::Option(option)
            }
            ValuePtr::Result(slot) => {
                let result = self.external_take(slot)?;

                let result = match result {
                    Ok(slot) => Ok(Box::new(self.value_take(slot)?)),
                    Err(slot) => Err(Box::new(self.value_take(slot)?)),
                };

                Value::Result(result)
            }
        });

        /// Convert into an owned array.
        fn value_take_array(vm: &mut Vm, values: Vec<ValuePtr>) -> Result<Vec<Value>, VmError> {
            let mut output = Vec::with_capacity(values.len());

            for value in values {
                output.push(vm.value_take(value)?);
            }

            Ok(output)
        }

        /// Convert into an owned tuple.
        fn value_take_tuple(vm: &mut Vm, values: &[ValuePtr]) -> Result<Box<[Value]>, VmError> {
            let mut output = Vec::with_capacity(values.len());

            for value in values.iter() {
                output.push(vm.value_take(*value)?);
            }

            Ok(output.into_boxed_slice())
        }

        /// Convert into an owned object.
        fn value_take_object(
            vm: &mut Vm,
            object: HashMap<String, ValuePtr>,
        ) -> Result<HashMap<String, Value>, VmError> {
            let mut output = HashMap::with_capacity(object.len());

            for (key, value) in object {
                output.insert(key, vm.value_take(value)?);
            }

            Ok(output)
        }
    }

    /// Convert the given ptr into a type-erase ValueRef.
    pub fn value_ref(&self, value: ValuePtr) -> Result<ValueRef<'_>, VmError> {
        Ok(match value {
            ValuePtr::Unit => ValueRef::Unit,
            ValuePtr::Integer(integer) => ValueRef::Integer(integer),
            ValuePtr::Float(float) => ValueRef::Float(float),
            ValuePtr::Bool(boolean) => ValueRef::Bool(boolean),
            ValuePtr::Char(c) => ValueRef::Char(c),
            ValuePtr::String(slot) => ValueRef::String(self.string_ref(slot)?),
            ValuePtr::StaticString(slot) => ValueRef::StaticString(self.unit.lookup_string(slot)?),
            ValuePtr::Array(slot) => {
                let array = self.array_ref(slot)?;
                ValueRef::Array(self.value_array_ref(&*array)?)
            }
            ValuePtr::Tuple(slot) => {
                let tuple = self.tuple_ref(slot)?;
                ValueRef::Tuple(self.value_tuple_ref(&*tuple)?)
            }
            ValuePtr::Object(slot) => {
                let object = self.object_ref(slot)?;
                ValueRef::Object(self.value_object_ref(&*object)?)
            }
            ValuePtr::External(slot) => ValueRef::External(self.external_ref_dyn(slot)?),
            ValuePtr::Type(ty) => ValueRef::Type(ty),
            ValuePtr::Fn(hash) => ValueRef::Fn(hash),
            ValuePtr::Future(slot) => {
                let future = self.external_ref(slot)?;
                ValueRef::Future(future)
            }
            ValuePtr::Option(slot) => {
                let option = self.option_ref(slot)?;

                let option = match *option {
                    Some(some) => Some(Box::new(self.value_ref(some)?)),
                    None => None,
                };

                ValueRef::Option(option)
            }
            ValuePtr::Result(slot) => {
                let result = self.result_ref(slot)?;

                let result = match *result {
                    Ok(ok) => Ok(Box::new(self.value_ref(ok)?)),
                    Err(err) => Err(Box::new(self.value_ref(err)?)),
                };

                ValueRef::Result(result)
            }
        })
    }

    /// Convert the given value pointers into an array.
    pub fn value_array_ref<'vm>(
        &'vm self,
        values: &[ValuePtr],
    ) -> Result<Vec<ValueRef<'vm>>, VmError> {
        let mut output = Vec::with_capacity(values.len());

        for value in values.iter().copied() {
            output.push(self.value_ref(value)?);
        }

        Ok(output)
    }

    /// Convert the given value pointers into a tuple.
    pub fn value_tuple_ref<'vm>(
        &'vm self,
        values: &[ValuePtr],
    ) -> Result<Box<[ValueRef<'vm>]>, VmError> {
        let mut output = Vec::with_capacity(values.len());

        for value in values.iter().copied() {
            output.push(self.value_ref(value)?);
        }

        Ok(output.into_boxed_slice())
    }

    /// Convert the given value pointers into an array.
    pub fn value_object_ref<'vm>(
        &'vm self,
        object: &HashMap<String, ValuePtr>,
    ) -> Result<HashMap<String, ValueRef<'vm>>, VmError> {
        let mut output = HashMap::with_capacity(object.len());

        for (key, value) in object.iter() {
            output.insert(key.to_owned(), self.value_ref(*value)?);
        }

        Ok(output)
    }

    /// Pop the last value on the stack and evaluate it as `T`.
    fn pop_decode<T>(&mut self) -> Result<T, VmError>
    where
        T: FromValue,
    {
        let value = self.stack.pop()?;

        let value = match T::from_value(value, self) {
            Ok(value) => value,
            Err(error) => {
                let type_info = value.type_info(self)?;

                return Err(VmError::StackConversionError {
                    error: Box::new(error),
                    from: type_info,
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
    fn value_ptr_eq(&self, a: ValuePtr, b: ValuePtr) -> Result<bool, VmError> {
        Ok(match (a, b) {
            (ValuePtr::Unit, ValuePtr::Unit) => true,
            (ValuePtr::Char(a), ValuePtr::Char(b)) => a == b,
            (ValuePtr::Bool(a), ValuePtr::Bool(b)) => a == b,
            (ValuePtr::Integer(a), ValuePtr::Integer(b)) => a == b,
            (ValuePtr::Float(a), ValuePtr::Float(b)) => a == b,
            (ValuePtr::Array(a), ValuePtr::Array(b)) => {
                let a = self.array_ref(a)?;
                let b = self.array_ref(b)?;

                if a.len() != b.len() {
                    return Ok(false);
                }

                for (a, b) in a.iter().copied().zip(b.iter().copied()) {
                    if !self.value_ptr_eq(a, b)? {
                        return Ok(false);
                    }
                }

                true
            }
            (ValuePtr::Object(a), ValuePtr::Object(b)) => {
                let a = self.object_ref(a)?;
                let b = self.object_ref(b)?;

                if a.len() != b.len() {
                    return Ok(false);
                }

                for (key, a) in a.iter() {
                    let b = match b.get(key) {
                        Some(b) => b,
                        None => return Ok(false),
                    };

                    if !self.value_ptr_eq(*a, *b)? {
                        return Ok(false);
                    }
                }

                true
            }
            (ValuePtr::String(a), ValuePtr::String(b)) => {
                let a = self.string_ref(a)?;
                let b = self.string_ref(b)?;
                *a == *b
            }
            (ValuePtr::StaticString(a), ValuePtr::String(b)) => {
                let a = self.unit.lookup_string(a)?;
                let b = self.string_ref(b)?;
                a == *b
            }
            (ValuePtr::String(a), ValuePtr::StaticString(b)) => {
                let a = self.string_ref(a)?;
                let b = self.unit.lookup_string(b)?;
                *a == b
            }
            // fast string comparison: exact string slot.
            (ValuePtr::StaticString(a), ValuePtr::StaticString(b)) => a == b,
            // fast external comparison by slot.
            (ValuePtr::External(a), ValuePtr::External(b)) => a == b,
            _ => false,
        })
    }

    /// Optimized equality implementation.
    #[inline]
    fn op_eq(&mut self) -> Result<(), VmError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        self.push(ValuePtr::Bool(self.value_ptr_eq(a, b)?));
        Ok(())
    }

    /// Optimized inequality implementation.
    #[inline]
    fn op_neq(&mut self) -> Result<(), VmError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        self.push(ValuePtr::Bool(!self.value_ptr_eq(a, b)?));
        Ok(())
    }

    /// Perform a jump operation.
    #[inline]
    fn op_jump(&mut self, offset: isize, update_ip: &mut bool) -> Result<(), VmError> {
        self.modify_ip(offset)?;
        *update_ip = false;
        Ok(())
    }

    /// Construct a new array.
    #[inline]
    fn op_array(&mut self, count: usize) -> Result<(), VmError> {
        let mut array = Vec::with_capacity(count);

        for _ in 0..count {
            array.push(self.stack.pop()?);
        }

        let value = self.array_allocate(array);
        self.push(value);
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
        let value = self.slot_allocate(tuple);
        self.push(ValuePtr::Tuple(value));
        Ok(())
    }

    #[inline]
    fn op_not(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let value = match value {
            ValuePtr::Bool(value) => ValuePtr::Bool(!value),
            other => {
                let operand = other.type_info(self)?;
                return Err(VmError::UnsupportedUnaryOperation { op: "!", operand });
            }
        };

        self.push(value);
        Ok(())
    }

    /// Perform an index set operation.
    #[inline]
    fn op_index_set(&mut self, context: &Context) -> Result<(), VmError> {
        let target = self.stack.pop()?;
        let index = self.stack.pop()?;
        let value = self.stack.pop()?;

        // This is a useful pattern.
        #[allow(clippy::never_loop)]
        while let ValuePtr::Object(target) = target {
            let index = match index {
                ValuePtr::String(index) => self.string_take(index)?,
                ValuePtr::StaticString(slot) => self.unit.lookup_string(slot)?.to_owned(),
                _ => break,
            };

            let mut object = self.object_mut(target)?;
            object.insert(index, value);
            return Ok(());
        }

        let ty = target.value_type(self)?;
        let hash = Hash::instance_function(ty, *crate::INDEX_SET);

        let handler = match context.lookup(hash) {
            Some(handler) => handler,
            None => {
                let target_type = target.type_info(self)?;
                let index_type = index.type_info(self)?;
                let value_type = value.type_info(self)?;

                return Err(VmError::UnsupportedIndexSet {
                    target_type,
                    index_type,
                    value_type,
                });
            }
        };

        self.push(value);
        self.push(index);
        self.push(target);
        handler(self, 2)?;
        Ok(())
    }

    /// Perform an index get operation.
    #[inline]
    fn op_index_get(&mut self, context: &Context) -> Result<(), VmError> {
        let target = self.stack.pop()?;
        let index = self.stack.pop()?;

        // This is a useful pattern.
        #[allow(clippy::never_loop)]
        while let ValuePtr::Object(target) = target {
            let string_ref;

            let index = match index {
                ValuePtr::String(index) => {
                    string_ref = self.string_ref(index)?;
                    string_ref.as_str()
                }
                ValuePtr::StaticString(slot) => self.unit.lookup_string(slot)?,
                _ => break,
            };

            let value = {
                let object = self.object_ref(target)?;
                object.get(index).copied().unwrap_or_default()
            };

            self.push(value);
            return Ok(());
        }

        let ty = target.value_type(self)?;
        let hash = Hash::instance_function(ty, *crate::INDEX_GET);

        let handler = match context.lookup(hash) {
            Some(handler) => handler,
            None => {
                let target_type = target.type_info(self)?;
                let index_type = index.type_info(self)?;

                return Err(VmError::UnsupportedIndexGet {
                    target_type,
                    index_type,
                });
            }
        };

        self.push(index);
        self.push(target);
        handler(self, 1)?;
        Ok(())
    }

    /// Perform an index get operation.
    #[inline]
    fn op_array_index_get(&mut self, index: usize) -> Result<(), VmError> {
        let target = self.stack.pop()?;

        let value = match target {
            ValuePtr::Array(slot) => {
                let array = self.array_ref(slot)?;

                match array.get(index).copied() {
                    Some(value) => value,
                    None => {
                        return Err(VmError::ArrayIndexMissing { index });
                    }
                }
            }
            target_type => {
                let target_type = target_type.type_info(self)?;
                return Err(VmError::UnsupportedArrayIndexGet { target_type });
            }
        };

        self.push(value);
        Ok(())
    }

    /// Perform an index get operation specialized for tuples.
    #[inline]
    fn op_tuple_index_get(&mut self, index: usize) -> Result<(), VmError> {
        let target = self.stack.pop()?;

        let value = match target {
            ValuePtr::Tuple(slot) => {
                let tuple = self.tuple_ref(slot)?;

                match tuple.get(index).copied() {
                    Some(value) => value,
                    None => {
                        return Err(VmError::TupleIndexMissing { index });
                    }
                }
            }
            ValuePtr::Result(slot) => {
                if index != 0 {
                    return Err(VmError::TupleIndexMissing { index });
                }

                let result = self.result_ref(slot)?;

                match *result {
                    Ok(ok) => ok,
                    Err(err) => err,
                }
            }
            ValuePtr::Option(slot) => {
                let option = self.option_ref(slot)?;

                match *option {
                    Some(some) => {
                        if index != 0 {
                            return Err(VmError::TupleIndexMissing { index });
                        }

                        some
                    }
                    None => {
                        return Err(VmError::TupleIndexMissing { index });
                    }
                }
            }
            target_type => {
                let target_type = target_type.type_info(self)?;
                return Err(VmError::UnsupportedTupleIndexGet { target_type });
            }
        };

        self.push(value);
        Ok(())
    }

    /// Perform a specialized index get operation on an object.
    #[inline]
    fn op_object_slot_index_get(&mut self, string_slot: usize) -> Result<(), VmError> {
        let target = self.stack.pop()?;

        let value = match target {
            ValuePtr::Object(slot) => {
                let index = self.unit.lookup_string(string_slot)?;

                let array = self.object_ref(slot)?;

                match array.get(index).copied() {
                    Some(value) => value,
                    None => {
                        return Err(VmError::ObjectIndexMissing { slot: string_slot });
                    }
                }
            }
            target_type => {
                let target_type = target_type.type_info(self)?;
                return Err(VmError::UnsupportedObjectSlotIndexGet { target_type });
            }
        };

        self.push(value);
        Ok(())
    }

    /// Operation to allocate an object.
    #[inline]
    fn op_object(&mut self, slot: usize) -> Result<(), VmError> {
        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or_else(|| VmError::MissingStaticObjectKeys { slot })?;

        let mut object = HashMap::with_capacity(keys.len());

        for key in keys {
            let value = self.stack.pop()?;
            object.insert(key.clone(), value);
        }

        let value = self.object_allocate(object);
        self.push(value);
        Ok(())
    }

    /// Optimize operation to perform string concatenation.
    #[inline]
    fn op_string_concat(&mut self, len: usize, size_hint: usize) -> Result<(), VmError> {
        let mut buf = String::with_capacity(size_hint);

        for _ in 0..len {
            let value = self.stack.pop()?;

            match value {
                ValuePtr::String(slot) => {
                    let string = self.string_ref(slot)?;
                    buf.push_str(&*string);
                }
                ValuePtr::StaticString(slot) => {
                    let string = self.unit.lookup_string(slot)?;
                    buf.push_str(string);
                }
                ValuePtr::Integer(integer) => {
                    let mut buffer = itoa::Buffer::new();
                    buf.push_str(buffer.format(integer));
                }
                ValuePtr::Float(float) => {
                    let mut buffer = ryu::Buffer::new();
                    buf.push_str(buffer.format(float));
                }
                actual => {
                    let actual = actual.type_info(self)?;

                    return Err(VmError::UnsupportedStringConcatArgument { actual });
                }
            }
        }

        let value = self.string_allocate(buf);
        self.push(value);
        Ok(())
    }

    #[inline]
    fn op_result_unwrap(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let result = match value {
            ValuePtr::Result(slot) => self.result_take(slot)?,
            actual => {
                return Err(VmError::ExpectedResult {
                    actual: actual.type_info(self)?,
                })
            }
        };

        let value = match result {
            Ok(ok) => ok,
            Err(error) => {
                return Err(VmError::ExpectedResultOk {
                    error: error.type_info(self)?,
                })
            }
        };

        self.stack.push(value);
        Ok(())
    }

    #[inline]
    fn op_option_unwrap(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let option = match value {
            ValuePtr::Option(slot) => self.option_take(slot)?,
            actual => {
                return Err(VmError::ExpectedOption {
                    actual: actual.type_info(self)?,
                })
            }
        };

        let value = match option {
            Some(some) => some,
            None => {
                return Err(VmError::ExpectedOptionSome);
            }
        };

        self.stack.push(value);
        Ok(())
    }

    #[inline]
    fn op_is(&mut self, context: &Context) -> Result<(), VmError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;

        match (a, b) {
            (a, ValuePtr::Type(hash)) => {
                let a = a.value_type(self)?;

                let type_info = context
                    .lookup_type(hash)
                    .ok_or_else(|| VmError::MissingType { hash })?;

                self.push(ValuePtr::Bool(a == type_info.value_type));
            }
            (a, b) => {
                let a = a.type_info(self)?;
                let b = b.type_info(self)?;

                return Err(VmError::UnsupportedIs {
                    value_type: a,
                    test_type: b,
                });
            }
        }

        Ok(())
    }

    /// Test if the top of the stack is an error.
    #[inline]
    fn op_is_err(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.push(ValuePtr::Bool(match value {
            ValuePtr::Result(slot) => self.result_ref(slot)?.is_err(),
            actual => {
                return Err(VmError::ExpectedResult {
                    actual: actual.type_info(self)?,
                })
            }
        }));

        Ok(())
    }

    /// Test if the top of the stack is none.
    ///
    /// TODO: optimize the layout of optional values to make this easier.
    #[inline]
    fn op_is_none(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.push(ValuePtr::Bool(match value {
            ValuePtr::Option(slot) => self.option_ref(slot)?.is_none(),
            actual => {
                return Err(VmError::ExpectedOption {
                    actual: actual.type_info(self)?,
                })
            }
        }));

        Ok(())
    }

    /// Operation associated with `and` instruction.
    #[inline]
    fn op_and(&mut self) -> Result<(), VmError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        let value = boolean_ops!(self, a && b);
        self.push(ValuePtr::Bool(value));
        Ok(())
    }

    /// Operation associated with `or` instruction.
    #[inline]
    fn op_or(&mut self) -> Result<(), VmError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        let value = boolean_ops!(self, a || b);
        self.push(ValuePtr::Bool(value));
        Ok(())
    }

    /// Test if the top of stack is equal to the string at the given static
    /// string location.
    #[inline]
    fn op_eq_static_string(&mut self, slot: usize) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        let string = self.unit.lookup_string(slot)?;

        self.stack.push(ValuePtr::Bool(match value {
            ValuePtr::String(slot) => {
                let actual = self.string_ref(slot)?;
                *actual == string
            }
            ValuePtr::StaticString(slot) => {
                let actual = self.unit.lookup_string(slot)?;
                actual == string
            }
            _ => false,
        }));

        Ok(())
    }

    #[inline]
    fn match_array<F>(&mut self, f: F) -> Result<(), VmError>
    where
        F: FnOnce(&Vec<ValuePtr>) -> bool,
    {
        let value = self.stack.pop()?;

        self.push(ValuePtr::Bool(match value {
            ValuePtr::Array(slot) => f(&*self.array_ref(slot)?),
            _ => false,
        }));

        Ok(())
    }

    #[inline]
    fn match_tuple<F>(&mut self, f: F) -> Result<(), VmError>
    where
        F: FnOnce(&[ValuePtr]) -> bool,
    {
        let value = self.stack.pop()?;

        self.push(ValuePtr::Bool(match value {
            ValuePtr::Tuple(slot) => f(&*self.tuple_ref(slot)?),
            ValuePtr::Result(slot) => {
                let result = self.result_ref(slot)?;

                match &*result {
                    Ok(ok) => f(&[*ok]),
                    Err(err) => f(&[*err]),
                }
            }
            ValuePtr::Option(slot) => {
                let option = self.option_ref(slot)?;

                match &*option {
                    Some(some) => f(&[*some]),
                    None => f(&[]),
                }
            }
            _ => false,
        }));

        Ok(())
    }

    #[inline]
    fn match_object<F>(&mut self, slot: usize, f: F) -> Result<(), VmError>
    where
        F: FnOnce(&HashMap<String, ValuePtr>, usize) -> bool,
    {
        let value = self.stack.pop()?;

        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or_else(|| VmError::MissingStaticObjectKeys { slot })?;

        let object = match value {
            ValuePtr::Object(slot) => self.object_ref(slot)?,
            _ => {
                self.push(ValuePtr::Bool(false));
                return Ok(());
            }
        };

        if !f(&*object, keys.len()) {
            self.push(ValuePtr::Bool(false));
            return Ok(());
        }

        let mut is_match = true;

        for key in keys {
            if !object.contains_key(key) {
                is_match = false;
                break;
            }
        }

        self.push(ValuePtr::Bool(is_match));
        Ok(())
    }

    /// Evaluate a single instruction.
    pub async fn run_for(
        &mut self,
        context: &Context,
        mut limit: Option<usize>,
    ) -> Result<(), VmError> {
        while !self.exited {
            let inst = *self
                .unit
                .instruction_at(self.ip)
                .ok_or_else(|| VmError::IpOutOfBounds)?;

            let mut update_ip = true;

            match inst {
                Inst::Not => {
                    self.op_not()?;
                }
                Inst::Add => {
                    let a = self.stack.pop()?;
                    let b = self.stack.pop()?;
                    numeric_ops!(self, context, crate::ADD, +, a.checked_add(b), Overflow);
                }
                Inst::AddAssign { offset } => {
                    let arg = self.stack.pop()?;
                    let value = self.stack.at_offset(offset)?;
                    let value = assign_ops! {
                        self, context, crate::ADD_ASSIGN, +, value.checked_add(arg), Overflow
                    };

                    *self.stack.at_offset_mut(offset)? = value;
                }
                Inst::Sub => {
                    let a = self.stack.pop()?;
                    let b = self.stack.pop()?;
                    numeric_ops!(self, context, crate::SUB, -, a.checked_sub(b), Underflow);
                }
                Inst::SubAssign { offset } => {
                    let arg = self.stack.pop()?;
                    let value = self.stack.at_offset(offset)?;
                    let value = assign_ops! {
                        self, context, crate::SUB_ASSIGN, -, value.checked_sub(arg), Underflow
                    };
                    *self.stack.at_offset_mut(offset)? = value;
                }
                Inst::Mul => {
                    let a = self.stack.pop()?;
                    let b = self.stack.pop()?;
                    numeric_ops!(self, context, crate::MUL, *, a.checked_mul(b), Overflow);
                }
                Inst::MulAssign { offset } => {
                    let arg = self.stack.pop()?;
                    let value = self.stack.at_offset(offset)?;
                    let value = assign_ops! {
                        self, context, crate::MUL_ASSIGN, *, value.checked_mul(arg), Overflow
                    };
                    *self.stack.at_offset_mut(offset)? = value;
                }
                Inst::Div => {
                    let a = self.stack.pop()?;
                    let b = self.stack.pop()?;
                    numeric_ops!(self, context, crate::DIV, /, a.checked_div(b), DivideByZero);
                }
                Inst::DivAssign { offset } => {
                    let arg = self.stack.pop()?;
                    let value = self.stack.at_offset(offset)?;
                    let value = assign_ops! {
                        self, context, crate::DIV_ASSIGN, /, value.checked_div(arg), DivideByZero
                    };
                    *self.stack.at_offset_mut(offset)? = value;
                }
                // NB: we inline function calls because it helps Rust optimize
                // the async plumbing.
                Inst::Call { hash, args } => {
                    call_fn!(self, hash, args, context, update_ip);
                }
                Inst::CallInstance { hash, args } => {
                    let instance = self.stack.peek()?;
                    let ty = instance.value_type(self)?;
                    let hash = Hash::instance_function(ty, hash);

                    match self.unit.lookup_offset(hash) {
                        Some(loc) => {
                            self.push_call_frame(loc, args)?;
                            update_ip = false;
                        }
                        None => {
                            let handler = match context.lookup(hash) {
                                Some(handler) => handler,
                                None => {
                                    return Err(VmError::MissingInstanceFunction {
                                        instance: instance.type_info(self)?,
                                        hash,
                                    });
                                }
                            };

                            handler(self, args)?;
                        }
                    }
                }
                Inst::CallFn { args } => {
                    let function = self.stack.pop()?;

                    let hash = match function {
                        ValuePtr::Fn(hash) => hash,
                        actual => {
                            let actual_type = actual.type_info(self)?;
                            return Err(VmError::UnsupportedCallFn { actual_type });
                        }
                    };

                    call_fn!(self, hash, args, context, update_ip);
                }
                Inst::LoadInstanceFn { hash } => {
                    let instance = self.stack.pop()?;
                    let ty = instance.value_type(self)?;
                    let hash = Hash::instance_function(ty, hash);
                    self.push(ValuePtr::Fn(hash));
                }
                Inst::IndexGet => {
                    self.op_index_get(context)?;
                }
                Inst::ArrayIndexGet { index } => {
                    self.op_array_index_get(index)?;
                }
                Inst::TupleIndexGet { index } => {
                    self.op_tuple_index_get(index)?;
                }
                Inst::ObjectSlotIndexGet { slot } => {
                    self.op_object_slot_index_get(slot)?;
                }
                Inst::IndexSet => {
                    self.op_index_set(context)?;
                }
                Inst::Return => {
                    let return_value = self.stack.pop()?;
                    self.exited = self.pop_call_frame()?;
                    self.push(return_value);
                }
                Inst::ReturnUnit => {
                    self.exited = self.pop_call_frame()?;
                    self.push(ValuePtr::Unit);
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
                Inst::Clean { count } => {
                    self.op_clean(count)?;
                }
                Inst::Integer { number } => {
                    self.push(ValuePtr::Integer(number));
                }
                Inst::Float { number } => {
                    self.push(ValuePtr::Float(number));
                }
                Inst::Copy { offset } => {
                    self.do_copy(offset)?;
                }
                Inst::Dup => {
                    self.do_dup()?;
                }
                Inst::Replace { offset } => {
                    self.do_replace(offset)?;
                }
                Inst::Gt => {
                    let a = self.stack.pop()?;
                    let b = self.stack.pop()?;
                    self.push(ValuePtr::Bool(primitive_ops!(self, a > b)));
                }
                Inst::Gte => {
                    let a = self.stack.pop()?;
                    let b = self.stack.pop()?;
                    self.push(ValuePtr::Bool(primitive_ops!(self, a >= b)));
                }
                Inst::Lt => {
                    let a = self.stack.pop()?;
                    let b = self.stack.pop()?;
                    self.push(ValuePtr::Bool(primitive_ops!(self, a < b)));
                }
                Inst::Lte => {
                    let a = self.stack.pop()?;
                    let b = self.stack.pop()?;
                    self.push(ValuePtr::Bool(primitive_ops!(self, a <= b)));
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
                    if pop!(self, Bool) {
                        self.modify_ip(offset)?;
                        update_ip = false;
                    }
                }
                Inst::JumpIfNot { offset } => {
                    if !pop!(self, Bool) {
                        self.modify_ip(offset)?;
                        update_ip = false;
                    }
                }
                Inst::JumpIfBranch { branch, offset } => {
                    if let Some(current) = self.branch {
                        if current == branch {
                            self.branch = None;
                            self.modify_ip(offset)?;
                            update_ip = false;
                        }
                    }
                }
                Inst::Unit => {
                    self.push(ValuePtr::Unit);
                }
                Inst::Bool { value } => {
                    self.push(ValuePtr::Bool(value));
                }
                Inst::Array { count } => {
                    self.op_array(count)?;
                }
                Inst::Tuple { count } => {
                    self.op_tuple(count)?;
                }
                Inst::Object { slot } => {
                    self.op_object(slot)?;
                }
                Inst::Type { hash } => {
                    self.push(ValuePtr::Type(hash));
                }
                Inst::Char { c } => {
                    self.push(ValuePtr::Char(c));
                }
                Inst::String { slot } => {
                    let string = self.unit.lookup_string(slot)?.to_owned();
                    // TODO: do something sneaky to only allocate the static string once.
                    let value = self.string_allocate(string);
                    self.push(value);
                }
                Inst::StaticString { slot } => {
                    self.push(ValuePtr::StaticString(slot));
                }
                Inst::StringConcat { len, size_hint } => {
                    self.op_string_concat(len, size_hint)?;
                }
                Inst::Is => {
                    self.op_is(context)?;
                }
                Inst::IsUnit => {
                    let value = self.stack.pop()?;
                    self.push(ValuePtr::Bool(matches!(value, ValuePtr::Unit)));
                }
                Inst::IsErr => {
                    self.op_is_err()?;
                }
                Inst::IsNone => {
                    self.op_is_none()?;
                }
                Inst::ResultUnwrap => {
                    self.op_result_unwrap()?;
                }
                Inst::OptionUnwrap => {
                    self.op_option_unwrap()?;
                }
                Inst::And => {
                    self.op_and()?;
                }
                Inst::Or => {
                    self.op_or()?;
                }
                Inst::EqCharacter { character } => {
                    let value = self.stack.pop()?;

                    self.push(ValuePtr::Bool(match value {
                        ValuePtr::Char(actual) => actual == character,
                        _ => false,
                    }));
                }
                Inst::EqInteger { integer } => {
                    let value = self.stack.pop()?;

                    self.push(ValuePtr::Bool(match value {
                        ValuePtr::Integer(actual) => actual == integer,
                        _ => false,
                    }));
                }
                Inst::EqStaticString { slot } => {
                    self.op_eq_static_string(slot)?;
                }
                Inst::MatchArray { len, exact } => {
                    if exact {
                        self.match_array(|array| array.len() == len)?;
                    } else {
                        self.match_array(|array| array.len() >= len)?;
                    }
                }
                Inst::MatchTuple { len, exact } => {
                    if exact {
                        self.match_tuple(|array| array.len() == len)?;
                    } else {
                        self.match_tuple(|array| array.len() >= len)?;
                    }
                }
                Inst::MatchObject { slot, exact } => {
                    if exact {
                        self.match_object(slot, |object, len| object.len() == len)?;
                    } else {
                        self.match_object(slot, |object, len| object.len() >= len)?;
                    }
                }
                Inst::Panic { reason } => {
                    return Err(VmError::Panic { reason });
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

impl fmt::Debug for Vm {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Vm")
            .field("ip", &self.ip)
            .field("exited", &self.exited)
            .field("stack", &self.stack)
            .field("call_frames", &self.call_frames)
            .field("slots", &"Slots")
            .finish()
    }
}

/// The task of a unit being run.
pub struct Task<'a, T> {
    /// The virtual machine of the task.
    vm: Vm,
    /// Functions collection associated with the task.
    context: Arc<Context>,
    /// Hold the type of the task.
    _marker: PhantomData<(&'a (), T)>,
}

impl<'a, T> Task<'a, T>
where
    T: FromValue,
{
    /// Access the underlying vm.
    pub fn vm(&self) -> &Vm {
        &self.vm
    }

    /// Run the given task to completion.
    pub async fn run_to_completion(&mut self) -> Result<T, VmError> {
        while !self.vm.exited {
            match self.vm.run_for(&*self.context, None).await {
                Ok(()) => (),
                Err(e) => return Err(e),
            }
        }

        match self.vm.pop_decode() {
            Ok(value) => Ok(value),
            Err(e) => Err(e),
        }
    }

    /// Step the given task until the return value is available.
    pub async fn step(&mut self) -> Result<Option<T>, VmError> {
        self.vm.run_for(&*self.context, Some(1)).await?;

        if self.vm.exited {
            let value = self.vm.pop_decode()?;
            return Ok(Some(value));
        }

        Ok(None)
    }
}

impl<T> Drop for Task<'_, T> {
    fn drop(&mut self) {
        // NB: this is critical for safety, since the slots might contain
        // references passed in externally which are tied to our lifetime ('a).
        self.vm.clear();
    }
}
