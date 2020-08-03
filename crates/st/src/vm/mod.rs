use crate::any::Any;
use crate::collections::HashMap;
use crate::context::{Context, Handler};
use crate::hash::Hash;
use crate::reflection::{FromValue, IntoArgs};
use crate::unit::Unit;
use crate::value::{Slot, Value, ValuePtr, ValueRef, ValueTypeInfo};
use anyhow::Result;
use slab::Slab;
use std::any::{type_name, TypeId};
use std::cell::UnsafeCell;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use thiserror::Error;

mod access;
mod inst;

use self::access::Access;
pub use self::access::{Mut, RawMutGuard, RawRefGuard, Ref};
pub use self::inst::Inst;

/// An error raised when interacting with types on the stack.
#[derive(Debug, Error)]
pub enum StackError {
    /// stack is empty
    #[error("stack is empty")]
    StackEmpty,
    /// Attempt to pop outside of current frame offset.
    #[error("attempted to pop beyond current stack frame `{frame}`")]
    PopOutOfBounds {
        /// Frame offset that we tried to pop.
        frame: usize,
    },
    /// No stack frames.
    #[error("stack frames are empty")]
    StackFramesEmpty,
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
    #[error("expected slot `{expected}`, but was `{actual}`")]
    UnexpectedSlotType {
        /// The type that was expected.
        expected: &'static str,
        /// The type that was found.
        actual: &'static str,
    },
    /// Error raised when we expected a boolean value.
    #[error("expected boolean value")]
    ExpectedBoolean,
    /// Error raised when we expected a char value.
    #[error("expected char value")]
    ExpectedChar,
    /// Error raised when an integer value was expected.
    #[error("expected integer value")]
    ExpectedInteger,
    /// Error raised when we expected a float value.
    #[error("expected float value")]
    ExpectedFloat,
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
    #[error("stack size `{stack_size}` starts before the current stack frame `{frame_at}`")]
    CorruptedStackFrame {
        /// The size of the stack.
        stack_size: usize,
        /// The location of the stack frame.
        frame_at: usize,
    },
}

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
    /// The virtual machine panicked for no specific reason.
    #[error("panicked `{mark}`")]
    Panic {
        /// Mark of the panic.
        mark: usize,
    },
    /// Error raised in a user-defined function.
    #[error("error in user-defined function")]
    UserError {
        /// Source error.
        #[from]
        error: crate::error::Error,
    },
    /// Failure to interact with the stack.
    #[error("failed to interact with the stack")]
    StackError {
        /// Source error.
        #[from]
        error: StackError,
    },
    /// Failure to lookup function.
    #[error("missing function with hash `{hash}`")]
    MissingFunction {
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
    /// Attempt to access out-of-bounds stack item.
    #[error("tried to access an out-of-bounds stack entry")]
    StackOutOfBounds,
    /// Indicates that a static string is missing for the given slot.
    #[error("static string slot `{slot}` does not exist")]
    MissingStaticString {
        /// Slot which is missing a static string.
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
        error: StackError,
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
        error: StackError,
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
        error: StackError,
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
    #[error("the array index get operation on `{target_type}` is not supported")]
    UnsupportedArrayIndexGet {
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
    /// Tried to fetch an index in an object that doesn't exist.
    #[error("missing index by static string slot `{slot}` in object")]
    ObjectIndexMissing {
        /// The static string slot corresponding to the index that is missing.
        slot: usize,
    },
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
            (ValuePtr::Unit, ValuePtr::Unit) => true,
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

/// Generate a primitive combination of operations.
macro_rules! numeric_ops {
    ($vm:expr, $a:ident $op:tt $b:ident) => {
        match ($a, $b) {
            (ValuePtr::Float($a), ValuePtr::Float($b)) => ValuePtr::Float($a $op $b),
            (ValuePtr::Integer($a), ValuePtr::Integer($b)) => ValuePtr::Integer($a $op $b),
            (lhs, rhs) => return Err(VmError::UnsupportedBinaryOperation {
                op: stringify!($op),
                lhs: lhs.type_info($vm)?,
                rhs: rhs.type_info($vm)?,
            }),
        }
    }
}

/// The holde of an external value.
#[derive(Debug)]
struct Holder {
    /// The generation this holder was created for.
    generation: usize,
    /// How the external is accessed (if it is accessed).
    /// This only happens during function calls, and the function callee is
    /// responsible for unwinding the access.
    access: Access,
    /// The value being held.
    value: Box<UnsafeCell<dyn Any>>,
}

/// A stack frame.
#[derive(Debug, Clone, Copy)]
struct Frame {
    /// The stored instruction pointer.
    ip: usize,
    /// The stored offset.
    frame_top: usize,
}

macro_rules! call_fn {
    ($s:expr, $hash:expr, $args:expr, $context:expr, $unit:expr, $update_ip:ident) => {
        let hash = $hash;

        match $unit.lookup_offset(hash) {
            Some(loc) => {
                $s.push_frame(loc, $args)?;
                $update_ip = false;
            }
            None => {
                let handler = $context
                    .lookup(hash)
                    .ok_or_else(|| VmError::MissingFunction { hash })?;

                let result = match handler {
                    Handler::Async(handler) => handler($s, $args).await,
                    Handler::Regular(handler) => handler($s, $args),
                };

                result?;
            }
        }
    };
}

macro_rules! impl_slot_functions {
    (
        $ty:ty,
        $slot:ident,
        $allocate_fn:ident,
        $ref_fn:ident,
        $mut_fn:ident,
        $take_fn:ident,
        $clone_fn:ident,
    ) => {
        /// Allocate a value and return its ptr.
        ///
        /// This operation can leak memory unless the returned slot is pushed onto
        /// the stack.
        ///
        /// Newly allocated items already have a refcount of 1. And should be
        /// pushed on the stack using [push], rather than
        /// [push.
        pub fn $allocate_fn(&mut self, value: $ty) -> ValuePtr {
            let generation = self.generation();
            ValuePtr::$slot(Slot::new(
                generation,
                self.internal_allocate(generation, value),
            ))
        }

        /// Get a reference of the value at the given slot.
        pub fn $ref_fn(&self, slot: Slot) -> Result<Ref<'_, $ty>, StackError> {
            self.external_ref::<$ty>(slot)
        }

        /// Get a cloned value from the given slot.
        pub fn $clone_fn(&self, slot: Slot) -> Result<$ty, StackError> {
            self.external_clone::<$ty>(slot)
        }

        /// Get a reference of the value at the given slot.
        pub fn $mut_fn(&self, slot: Slot) -> Result<Mut<'_, $ty>, StackError> {
            self.external_mut::<$ty>(slot)
        }

        /// Take the value at the given slot.
        ///
        /// After taking the value, the caller is responsible for deallocating it.
        pub fn $take_fn(&mut self, slot: Slot) -> Result<$ty, StackError> {
            self.external_take::<$ty>(slot)
        }
    };
}

/// A stack which references variables indirectly from a slab.
pub struct Vm {
    /// The current instruction pointer.
    ip: usize,
    /// The top of the current frame.
    frame_top: usize,
    /// We have exited from the last frame.
    exited: bool,
    /// The current stack of values.
    stack: Vec<ValuePtr>,
    /// Frames relative to the stack.
    frames: Vec<Frame>,
    /// Slots with external values.
    slots: Slab<Holder>,
    /// Generation used for allocated objects.
    generation: usize,
    /// A temporary buffer to hold values.
    temp: Vec<ValuePtr>,
}

impl Vm {
    /// Construct a new ST virtual machine.
    pub fn new() -> Self {
        Self {
            ip: 0,
            frame_top: 0,
            exited: false,
            stack: Vec::new(),
            frames: Vec::new(),
            slots: Slab::new(),
            generation: 0,
            temp: Vec::with_capacity(16),
        }
    }

    /// Reset this virtual machine, freeing all memory used.
    pub fn clear(&mut self) {
        self.ip = 0;
        self.frame_top = 0;
        self.exited = false;
        self.stack.clear();
        self.frames.clear();
        self.slots.clear();
        self.generation = 0;
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
    ) -> impl Iterator<Item = (ValuePtr, Result<ValueRef<'_>, StackError>)> + '_ {
        let mut it = self.stack.iter().copied();

        std::iter::from_fn(move || {
            let value_ref = it.next()?;
            let value = self.value_ref(value_ref);
            Some((value_ref, value))
        })
    }

    /// Call the given function in the given compilation unit.
    pub fn call_function<'a, A, T, I>(
        &'a mut self,
        context: &'a Context,
        unit: &'a Unit,
        name: I,
        args: A,
    ) -> Result<Task<'a, T>, VmError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
        A: IntoArgs,
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

        args.into_args(self)?;

        self.ip = function.offset;
        self.frame_top = 0;

        Ok(Task {
            vm: self,
            context,
            unit,
            _marker: PhantomData,
        })
    }

    /// Run the given program on the virtual machine.
    pub fn run<'a, T>(&'a mut self, context: &'a Context, unit: &'a Unit) -> Task<'a, T>
    where
        T: FromValue,
    {
        Task {
            vm: self,
            context,
            unit,
            _marker: PhantomData,
        }
    }

    /// Push an unmanaged reference.
    ///
    /// The reference count of the value being referenced won't be modified.
    pub fn push(&mut self, value: ValuePtr) {
        self.stack.push(value);
    }

    /// Pop a reference to a value from the stack.
    pub fn pop(&mut self) -> Result<ValuePtr, StackError> {
        if self.stack.len() == self.frame_top {
            return Err(StackError::PopOutOfBounds {
                frame: self.frame_top,
            });
        }

        self.stack.pop().ok_or_else(|| StackError::StackEmpty)
    }

    /// Pop a number of values from the stack.
    fn op_popn(&mut self, n: usize) -> Result<(), StackError> {
        if self.stack.len().saturating_sub(self.frame_top) < n {
            return Err(StackError::PopOutOfBounds {
                frame: self.frame_top,
            });
        }

        for _ in 0..n {
            self.stack.pop().ok_or_else(|| StackError::StackEmpty)?;
        }

        Ok(())
    }

    /// Pop a number of values from the stack, while preserving the top of the
    /// stack.
    fn op_clean(&mut self, n: usize) -> Result<(), StackError> {
        let value = self.pop()?;
        self.op_popn(n)?;
        self.push(value);
        Ok(())
    }

    /// Peek the top of the stack.
    fn peek(&mut self) -> Result<ValuePtr, StackError> {
        self.stack
            .last()
            .copied()
            .ok_or_else(|| StackError::StackEmpty)
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    fn do_copy(&mut self, offset: usize) -> Result<(), VmError> {
        let value = self
            .frame_top
            .checked_add(offset)
            .and_then(|n| self.stack.get(n).copied())
            .ok_or_else(|| VmError::StackOutOfBounds)?;

        self.stack.push(value);
        Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    fn do_replace(&mut self, offset: usize) -> Result<(), VmError> {
        let mut value = self.stack.pop().ok_or_else(|| VmError::StackOutOfBounds)?;

        let stack_value = self
            .frame_top
            .checked_add(offset)
            .and_then(|n| self.stack.get_mut(n))
            .ok_or_else(|| VmError::StackOutOfBounds)?;

        mem::swap(stack_value, &mut value);
        Ok(())
    }

    /// Push a new call frame.
    fn push_frame(&mut self, new_ip: usize, args: usize) -> Result<(), VmError> {
        let offset = self
            .stack
            .len()
            .checked_sub(args)
            .ok_or_else(|| VmError::StackOutOfBounds)?;

        self.frames.push(Frame {
            ip: self.ip,
            frame_top: self.frame_top,
        });

        self.frame_top = offset;
        self.ip = new_ip;
        Ok(())
    }

    /// Pop a call frame and return it.
    fn pop_frame(&mut self) -> Result<bool, StackError> {
        let frame = match self.frames.pop() {
            Some(frame) => frame,
            None => return Ok(true),
        };

        // Assert that the stack has been restored to the current stack top.
        if self.stack.len() != self.frame_top {
            return Err(StackError::CorruptedStackFrame {
                stack_size: self.stack.len(),
                frame_at: self.frame_top,
            });
        }

        self.frame_top = frame.frame_top;
        self.ip = frame.ip;
        Ok(false)
    }

    fn internal_allocate<T: Any>(&mut self, generation: usize, value: T) -> usize {
        self.slots.insert(Holder {
            generation,
            access: Access::default(),
            value: Box::new(UnsafeCell::new(value)),
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
    pub fn external_allocate<T: Any>(&mut self, value: T) -> ValuePtr {
        let generation = self.generation();
        ValuePtr::External(Slot::new(
            generation,
            self.internal_allocate(generation, value),
        ))
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

    /// Get a reference of the external value of the given type and the given
    /// slot.
    pub fn external_ref<T: Any>(&self, slot: Slot) -> Result<Ref<'_, T>, StackError> {
        let holder = self
            .slots
            .get(slot.into_usize())
            .filter(|h| h.generation == slot.into_generation())
            .ok_or_else(|| StackError::SlotMissing { slot })?;

        holder.access.shared(slot)?;

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        unsafe {
            let value = match (*holder.value.get()).as_ptr(TypeId::of::<T>()) {
                Some(value) => value,
                None => {
                    let actual = (*holder.value.get()).type_name();

                    // NB: Immediately unshare because the cast failed and we
                    // won't be maintaining access to the type.
                    holder.access.release_shared();

                    return Err(StackError::UnexpectedSlotType {
                        expected: type_name::<T>(),
                        actual,
                    });
                }
            };

            Ok(Ref {
                value: &*(value as *const T),
                raw: RawRefGuard {
                    access: &holder.access,
                },
            })
        }
    }

    /// Get a mutable reference of the external value of the given type and the
    /// given slot.
    ///
    /// Mark the given value as mutably used, preventing it from being used
    /// again.
    pub fn external_mut<T: Any>(&self, slot: Slot) -> Result<Mut<'_, T>, StackError> {
        let holder = self
            .slots
            .get(slot.into_usize())
            .filter(|h| h.generation == slot.into_generation())
            .ok_or_else(|| StackError::SlotMissing { slot })?;

        holder.access.exclusive(slot)?;

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        unsafe {
            let value = match (*holder.value.get()).as_mut_ptr(TypeId::of::<T>()) {
                Some(value) => value,
                None => {
                    let actual = (*holder.value.get()).type_name();

                    // NB: Immediately unshare because the cast failed and we
                    // won't be maintaining access to the type.
                    holder.access.release_exclusive();

                    return Err(StackError::UnexpectedSlotType {
                        expected: type_name::<T>(),
                        actual,
                    });
                }
            };

            Ok(Mut {
                value: &mut *(value as *mut T),
                raw: RawMutGuard {
                    access: &holder.access,
                },
            })
        }
    }

    /// Get a clone of the given external.
    pub fn external_clone<T: Clone + Any>(&self, slot: Slot) -> Result<T, StackError> {
        let holder = self
            .slots
            .get(slot.into_usize())
            .filter(|h| h.generation == slot.into_generation())
            .ok_or_else(|| StackError::SlotMissing { slot })?;

        // NB: we don't need a guard here since we're only using the reference
        // for the duration of this function.
        holder.access.test_shared(slot)?;

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        unsafe {
            let value = match (*holder.value.get()).as_ptr(TypeId::of::<T>()) {
                Some(value) => &*(value as *const T),
                None => {
                    let actual = (*holder.value.get()).type_name();

                    return Err(StackError::UnexpectedSlotType {
                        expected: type_name::<T>(),
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
    fn convert_value<T>(value: Box<UnsafeCell<dyn Any>>) -> Result<T, Box<UnsafeCell<dyn Any>>>
    where
        T: 'static,
    {
        // Safety: The conversion is fully checked through the invariants
        // provided by our custom `Any` implementaiton.
        //
        // `as_mut_ptr` ensures that the type of the boxed value matches the
        // expected type.
        unsafe {
            let value = Box::into_raw(value);

            if let Some(ptr) = (*(*value).get()).as_mut_ptr(TypeId::of::<T>()) {
                return Ok(*Box::from_raw(ptr as *mut T));
            }

            Err(Box::from_raw(value))
        }
    }

    /// Take an external value from the virtual machine by its slot.
    pub fn external_take<T>(&mut self, slot: Slot) -> Result<T, StackError>
    where
        T: Any,
    {
        let pos = slot.into_usize();

        // NB: don't need to perform a runtime check because this function
        // requires exclusive access to the virtual machine, at which point it's
        // impossible for live references to slots to be out unless unsafe
        // functions have been used in an unsound manner.
        if self
            .slots
            .get(slot.into_usize())
            .filter(|h| h.generation == slot.into_generation())
            .is_none()
        {
            return Err(StackError::SlotMissing { slot });
        }

        let holder = self.slots.remove(pos);

        match Self::convert_value(holder.value) {
            Ok(value) => return Ok(value),
            Err(value) => {
                let actual = unsafe { (*value.get()).type_name() };

                Err(StackError::UnexpectedSlotType {
                    expected: type_name::<T>(),
                    actual,
                })
            }
        }
    }

    fn external_with_dyn<F, T>(&self, slot: Slot, f: F) -> Result<T, StackError>
    where
        F: FnOnce(&dyn Any) -> T,
    {
        let holder = self
            .slots
            .get(slot.into_usize())
            .filter(|h| h.generation == slot.into_generation())
            .ok_or_else(|| StackError::SlotMissing { slot })?;

        holder.access.test_shared(slot)?;

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        Ok(f(unsafe { &*holder.value.get() }))
    }

    /// Get a reference of the external value of the given type and the given
    /// slot.
    pub fn external_ref_dyn(&self, slot: Slot) -> Result<Ref<'_, dyn Any>, StackError> {
        let holder = self
            .slots
            .get(slot.into_usize())
            .filter(|h| h.generation == slot.into_generation())
            .ok_or_else(|| StackError::SlotMissing { slot })?;

        holder.access.shared(slot)?;

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        Ok(Ref {
            value: unsafe { &*holder.value.get() },
            raw: RawRefGuard {
                access: &holder.access,
            },
        })
    }

    /// Take an external value by dyn, assuming you have exlusive access to it.
    pub fn external_take_dyn(&mut self, slot: Slot) -> Result<Box<dyn Any>, StackError> {
        let pos = slot.into_usize();

        if self
            .slots
            .get(pos)
            .filter(|h| h.generation == slot.into_generation())
            .is_none()
        {
            return Err(StackError::SlotMissing { slot });
        }

        let holder = self.slots.remove(pos);

        // Safety: We have the necessary level of ownership to guarantee that
        // the reference cast is safe, and we wrap the return value in a
        // guard which ensures the needed access level.
        unsafe {
            let value = Box::into_raw(holder.value);
            return Ok(Box::from_raw((*value).get()));
        }
    }

    /// Access the type name of the slot.
    pub fn slot_type_name(&self, slot: Slot) -> Result<&'static str, StackError> {
        self.external_with_dyn(slot, |e| e.type_name())
    }

    /// Access the type id of the slot.
    pub fn slot_type_id(&self, slot: Slot) -> Result<TypeId, StackError> {
        self.external_with_dyn(slot, |e| e.type_id())
    }

    /// Convert a value reference into an owned value.
    pub fn value_take(&mut self, value: ValuePtr) -> Result<Value, StackError> {
        return Ok(match value {
            ValuePtr::Unit => Value::Unit,
            ValuePtr::Integer(integer) => Value::Integer(integer),
            ValuePtr::Float(float) => Value::Float(float),
            ValuePtr::Bool(boolean) => Value::Bool(boolean),
            ValuePtr::Char(c) => Value::Char(c),
            ValuePtr::String(slot) => Value::String(self.string_take(slot)?),
            ValuePtr::Array(slot) => {
                let array = self.array_take(slot)?;
                Value::Array(value_take_array(self, array)?)
            }
            ValuePtr::Object(slot) => {
                let object = self.object_take(slot)?;
                Value::Object(value_take_object(self, object)?)
            }
            ValuePtr::External(slot) => Value::External(self.external_take_dyn(slot)?),
            ValuePtr::Type(ty) => Value::Type(ty),
            ValuePtr::Ptr(ptr) => Value::Ptr(ptr),
            ValuePtr::Fn(hash) => Value::Fn(hash),
        });

        /// Convert into an owned array.
        fn value_take_array(vm: &mut Vm, values: Vec<ValuePtr>) -> Result<Vec<Value>, StackError> {
            let mut output = Vec::with_capacity(values.len());

            for value in values {
                output.push(vm.value_take(value)?);
            }

            Ok(output)
        }

        /// Convert into an owned object.
        fn value_take_object(
            vm: &mut Vm,
            object: HashMap<String, ValuePtr>,
        ) -> Result<HashMap<String, Value>, StackError> {
            let mut output = HashMap::with_capacity(object.len());

            for (key, value) in object {
                output.insert(key, vm.value_take(value)?);
            }

            Ok(output)
        }
    }

    /// Convert the given ptr into a type-erase ValueRef.
    pub fn value_ref(&self, value: ValuePtr) -> Result<ValueRef<'_>, StackError> {
        return Ok(match value {
            ValuePtr::Unit => ValueRef::Unit,
            ValuePtr::Integer(integer) => ValueRef::Integer(integer),
            ValuePtr::Float(float) => ValueRef::Float(float),
            ValuePtr::Bool(boolean) => ValueRef::Bool(boolean),
            ValuePtr::Char(c) => ValueRef::Char(c),
            ValuePtr::String(slot) => ValueRef::String(self.string_ref(slot)?),
            ValuePtr::Array(slot) => {
                let array = self.array_ref(slot)?;
                ValueRef::Array(self.value_array_ref(&*array)?)
            }
            ValuePtr::Object(slot) => {
                let object = self.object_ref(slot)?;
                ValueRef::Object(self.value_object_ref(&*object)?)
            }
            ValuePtr::External(slot) => ValueRef::External(self.external_ref_dyn(slot)?),
            ValuePtr::Type(ty) => ValueRef::Type(ty),
            ValuePtr::Ptr(ptr) => ValueRef::Ptr(ptr),
            ValuePtr::Fn(hash) => ValueRef::Fn(hash),
        });
    }

    /// Convert the given value pointers into an array.
    pub fn value_array_ref(&self, values: &[ValuePtr]) -> Result<Vec<ValueRef<'_>>, StackError> {
        let mut output = Vec::with_capacity(values.len());

        for value in values.iter().copied() {
            output.push(self.value_ref(value)?);
        }

        Ok(output)
    }

    /// Convert the given value pointers into an array.
    pub fn value_object_ref(
        &self,
        object: &HashMap<String, ValuePtr>,
    ) -> Result<HashMap<String, ValueRef<'_>>, StackError> {
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
        let value = self.pop()?;

        let value = match T::from_value(value, self) {
            Ok(value) => value,
            Err(error) => {
                let type_info = value.type_info(self)?;

                return Err(VmError::StackConversionError {
                    error,
                    from: type_info,
                    to: type_name::<T>(),
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
            (ValuePtr::External(a), ValuePtr::External(b)) => a == b,
            _ => false,
        })
    }

    /// Optimized equality implementation.
    #[inline]
    fn op_eq(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(ValuePtr::Bool(self.value_ptr_eq(a, b)?));
        Ok(())
    }

    /// Optimized inequality implementation.
    #[inline]
    fn op_neq(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
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

    #[inline]
    fn op_not(&mut self) -> Result<(), VmError> {
        let value = self.pop()?;

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

    /// Implementation of the add operation.
    #[inline]
    fn op_add(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;

        match (a, b) {
            (ValuePtr::Float(a), ValuePtr::Float(b)) => {
                self.push(ValuePtr::Float(a + b));
                return Ok(());
            }
            (ValuePtr::Integer(a), ValuePtr::Integer(b)) => {
                self.push(ValuePtr::Integer(a + b));
                return Ok(());
            }
            (ValuePtr::String(a), ValuePtr::String(b)) => {
                let string = {
                    let a = self.string_ref(a)?;
                    let b = self.string_ref(b)?;
                    let mut string = String::with_capacity(a.len() + b.len());
                    string.push_str(a.as_str());
                    string.push_str(b.as_str());
                    string
                };

                let value = self.string_allocate(string);
                self.push(value);
                return Ok(());
            }
            _ => (),
        };

        Err(VmError::UnsupportedBinaryOperation {
            op: "+",
            lhs: a.type_info(self)?,
            rhs: b.type_info(self)?,
        })
    }

    #[inline]
    fn op_sub(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(numeric_ops!(self, a - b));
        Ok(())
    }

    #[inline]
    fn op_div(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(numeric_ops!(self, a / b));
        Ok(())
    }

    #[inline]
    fn op_mul(&mut self) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(numeric_ops!(self, a * b));
        Ok(())
    }

    /// Perform an index get operation.
    #[inline]
    fn op_index_get(&mut self) -> Result<(), VmError> {
        let target = self.pop()?;
        let index = self.pop()?;

        match (target, index) {
            (ValuePtr::Object(target), ValuePtr::String(index)) => {
                let value = {
                    let object = self.object_ref(target)?;
                    let index = self.string_ref(index)?;
                    object.get(&*index).copied().unwrap_or_default()
                };

                self.push(value);
                return Ok(());
            }
            _ => (),
        }

        let target_type = target.type_info(self)?;
        let index_type = index.type_info(self)?;

        Err(VmError::UnsupportedIndexGet {
            target_type,
            index_type,
        })
    }

    /// Perform an index get operation.
    #[inline]
    fn op_array_index_get(&mut self, index: usize) -> Result<(), VmError> {
        let target = self.pop()?;

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

    /// Perform a specialized index get operation on an object.
    #[inline]
    fn op_object_slot_index_get(&mut self, string_slot: usize, unit: &Unit) -> Result<(), VmError> {
        let target = self.pop()?;

        let value = match target {
            ValuePtr::Object(slot) => {
                let index = unit
                    .lookup_string(string_slot)
                    .ok_or_else(|| VmError::MissingStaticString { slot: string_slot })?;

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

    /// Perform an index set operation.
    #[inline]
    fn op_index_set(&mut self) -> Result<(), VmError> {
        let target = self.pop()?;
        let index = self.pop()?;
        let value = self.pop()?;

        match (target, index) {
            (ValuePtr::Object(target), ValuePtr::String(index)) => {
                let index = self.string_take(index)?;
                let mut object = self.object_mut(target)?;
                object.insert(index, value);
                return Ok(());
            }
            _ => (),
        }

        let target_type = target.type_info(self)?;
        let index_type = index.type_info(self)?;
        let value_type = value.type_info(self)?;

        Err(VmError::UnsupportedIndexSet {
            target_type,
            index_type,
            value_type,
        })
    }

    /// Implementation of the `replace-ref` instruction.
    #[inline]
    fn op_replace_deref(&mut self) -> Result<(), VmError> {
        let target = self.pop()?;
        let value = self.pop()?;

        let target_ptr = match target {
            ValuePtr::Ptr(ptr) => ptr,
            _ => {
                let target_type = target.type_info(self)?;
                let value_type = value.type_info(self)?;

                return Err(VmError::UnsupportedReplaceDeref {
                    target_type,
                    value_type,
                });
            }
        };

        // NB: Validate the value being assigned. This is the only instruction
        // allowed to reassign pointer.
        //
        // At creation time it is guaranteed to only point to a lower location,
        // making sure that a pointer is always popped before the value it
        // points to.
        //
        // Therefore, a pointer type cannot be reassigned to a location on the
        // stack that to a higher memory location to guarantee that it points to
        // valid data.
        match value {
            ValuePtr::Ptr(value_ptr) => {
                if value_ptr > target_ptr {
                    return Err(VmError::IllegalPtrReplace {
                        target_ptr,
                        value_ptr,
                    });
                }
            }
            _ => (),
        }

        let replace = self
            .stack
            .get_mut(target_ptr)
            .ok_or_else(|| VmError::StackOutOfBounds)?;

        *replace = value;
        Ok(())
    }

    #[inline]
    fn op_is(&mut self, context: &Context) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;

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

    #[inline]
    fn op_eq_static_string(&mut self, slot: usize, unit: &Unit) -> Result<(), VmError> {
        let string = unit
            .lookup_string(slot)
            .ok_or_else(|| VmError::MissingStaticString { slot })?;

        let value = self.pop()?;

        self.push(ValuePtr::Bool(match value {
            ValuePtr::String(slot) => {
                let actual = self.string_ref(slot)?;
                *actual == string
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
        let value = self.pop()?;

        self.push(ValuePtr::Bool(match value {
            ValuePtr::Array(slot) => f(&*self.array_ref(slot)?),
            _ => false,
        }));

        Ok(())
    }

    #[inline]
    fn match_object<F>(&mut self, len: usize, f: F) -> Result<(), VmError>
    where
        F: FnOnce(&HashMap<String, ValuePtr>) -> bool,
    {
        let value = self.pop()?;

        self.temp.clear();

        for _ in 0..len {
            let value = self.pop()?;
            self.temp.push(value);
        }

        let object = match value {
            ValuePtr::Object(slot) => self.object_ref(slot)?,
            _ => {
                self.push(ValuePtr::Bool(false));
                return Ok(());
            }
        };

        if !f(&*object) {
            self.push(ValuePtr::Bool(false));
            return Ok(());
        }

        let mut is_match = true;

        for value in self.temp.iter().copied() {
            let string = match value {
                ValuePtr::String(slot) => self.string_ref(slot)?,
                actual => {
                    let actual = actual.type_info(self)?;
                    return Err(VmError::UnsupportedObjectKey { actual });
                }
            };

            if !object.contains_key(&*string) {
                is_match = false;
                break;
            }
        }

        self.push(ValuePtr::Bool(is_match));
        Ok(())
    }

    #[inline]
    fn op_ptr(&mut self, offset: usize) -> Result<(), VmError> {
        let ptr = self
            .frame_top
            .checked_add(offset)
            .ok_or_else(|| VmError::StackOutOfBounds)?;

        self.push(ValuePtr::Ptr(ptr));
        Ok(())
    }

    /// Deref the value on the stack and push it.
    #[inline]
    fn op_deref(&mut self) -> Result<(), VmError> {
        let target = self.pop()?;

        let ptr = match target {
            ValuePtr::Ptr(ptr) => ptr,
            actual => {
                let actual_type = actual.type_info(self)?;
                return Err(VmError::UnsupportedDeref { actual_type });
            }
        };

        let value = self
            .stack
            .get(ptr)
            .copied()
            .ok_or_else(|| VmError::StackOutOfBounds)?;

        self.push(value);
        Ok(())
    }

    /// Evaluate a single instruction.
    pub async fn run_for(
        &mut self,
        context: &Context,
        unit: &Unit,
        mut limit: Option<usize>,
    ) -> Result<(), VmError> {
        while !self.exited {
            let inst = unit
                .instruction_at(self.ip)
                .ok_or_else(|| VmError::IpOutOfBounds)?;

            let mut update_ip = true;

            match inst {
                Inst::Not => {
                    self.op_not()?;
                }
                Inst::Add => {
                    self.op_add()?;
                }
                Inst::Sub => {
                    self.op_sub()?;
                }
                Inst::Div => {
                    self.op_div()?;
                }
                Inst::Mul => {
                    self.op_mul()?;
                }
                // NB: we inline function calls because it helps Rust optimize
                // the async plumbing.
                Inst::Call { hash, args } => {
                    call_fn!(self, *hash, *args, context, unit, update_ip);
                }
                Inst::CallInstance { hash, args } => {
                    let instance = self.peek()?;
                    let ty = instance.value_type(self)?;
                    let hash = Hash::instance_function(ty, *hash);

                    call_fn!(self, hash, *args, context, unit, update_ip);
                }
                Inst::CallFn { args } => {
                    let function = self.pop()?;

                    let hash = match function {
                        ValuePtr::Fn(hash) => hash,
                        actual => {
                            let actual_type = actual.type_info(self)?;
                            return Err(VmError::UnsupportedCallFn { actual_type });
                        }
                    };

                    call_fn!(self, hash, *args, context, unit, update_ip);
                }
                Inst::LoadInstanceFn { hash } => {
                    let instance = self.pop()?;
                    let ty = instance.value_type(self)?;
                    let hash = Hash::instance_function(ty, *hash);
                    self.push(ValuePtr::Fn(hash));
                }
                Inst::IndexGet => {
                    self.op_index_get()?;
                }
                Inst::ArrayIndexGet { index } => {
                    self.op_array_index_get(*index)?;
                }
                Inst::ObjectSlotIndexGet { slot } => {
                    self.op_object_slot_index_get(*slot, unit)?;
                }
                Inst::IndexSet => {
                    self.op_index_set()?;
                }
                Inst::Return => {
                    let return_value = self.pop()?;
                    self.exited = self.pop_frame()?;
                    self.push(return_value);
                }
                Inst::ReturnUnit => {
                    self.exited = self.pop_frame()?;
                    self.push(ValuePtr::Unit);
                }
                Inst::Pop => {
                    self.pop()?;
                }
                Inst::PopN { count } => {
                    self.op_popn(*count)?;
                }
                Inst::Clean { count } => {
                    self.op_clean(*count)?;
                }
                Inst::Integer { number } => {
                    self.push(ValuePtr::Integer(*number));
                }
                Inst::Float { number } => {
                    self.push(ValuePtr::Float(*number));
                }
                Inst::Copy { offset } => {
                    self.do_copy(*offset)?;
                }
                Inst::Replace { offset } => {
                    self.do_replace(*offset)?;
                }
                Inst::ReplaceDeref => {
                    self.op_replace_deref()?;
                }
                Inst::Gt => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(ValuePtr::Bool(primitive_ops!(self, a > b)));
                }
                Inst::Gte => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(ValuePtr::Bool(primitive_ops!(self, a >= b)));
                }
                Inst::Lt => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(ValuePtr::Bool(primitive_ops!(self, a < b)));
                }
                Inst::Lte => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(ValuePtr::Bool(primitive_ops!(self, a <= b)));
                }
                Inst::Eq => {
                    self.op_eq()?;
                }
                Inst::Neq => {
                    self.op_neq()?;
                }
                Inst::Jump { offset } => {
                    self.op_jump(*offset, &mut update_ip)?;
                }
                Inst::JumpIf { offset } => {
                    if pop!(self, Bool) {
                        self.modify_ip(*offset)?;
                        update_ip = false;
                    }
                }
                Inst::JumpIfNot { offset } => {
                    if !pop!(self, Bool) {
                        self.modify_ip(*offset)?;
                        update_ip = false;
                    }
                }
                Inst::Unit => {
                    self.push(ValuePtr::Unit);
                }
                Inst::Bool { value } => {
                    self.push(ValuePtr::Bool(*value));
                }
                Inst::Array { count } => {
                    let mut array = Vec::with_capacity(*count);

                    for _ in 0..*count {
                        array.push(self.stack.pop().ok_or_else(|| StackError::StackEmpty)?);
                    }

                    let value = self.array_allocate(array);
                    self.push(value);
                }
                Inst::Object { count } => {
                    let mut object = HashMap::with_capacity(*count);

                    for _ in 0..*count {
                        let key = self.pop_decode()?;
                        let value = self.pop()?;
                        object.insert(key, value);
                    }

                    let value = self.object_allocate(object);
                    self.push(value);
                }
                Inst::Type { hash } => {
                    self.push(ValuePtr::Type(*hash));
                }
                Inst::Char { c } => {
                    self.push(ValuePtr::Char(*c));
                }
                Inst::String { slot } => {
                    let string = unit
                        .lookup_string(*slot)
                        .ok_or_else(|| VmError::MissingStaticString { slot: *slot })?;
                    // TODO: do something sneaky to only allocate the static string once.
                    let value = self.string_allocate(string.to_owned());
                    self.push(value);
                }
                Inst::Is => {
                    self.op_is(context)?;
                }
                Inst::IsUnit => {
                    let value = self.pop()?;

                    self.push(ValuePtr::Bool(match value {
                        ValuePtr::Unit => true,
                        _ => false,
                    }));
                }
                Inst::EqCharacter { character } => {
                    let value = self.pop()?;

                    self.push(ValuePtr::Bool(match value {
                        ValuePtr::Char(actual) => actual == *character,
                        _ => false,
                    }));
                }
                Inst::EqInteger { integer } => {
                    let value = self.pop()?;

                    self.push(ValuePtr::Bool(match value {
                        ValuePtr::Integer(actual) => actual == *integer,
                        _ => false,
                    }));
                }
                Inst::EqStaticString { slot } => {
                    self.op_eq_static_string(*slot, unit)?;
                }
                Inst::MatchArray { len, exact } => {
                    let len = *len;

                    if *exact {
                        self.match_array(|array| array.len() == len)?;
                    } else {
                        self.match_array(|array| array.len() >= len)?;
                    }
                }
                Inst::MatchObject { len, exact } => {
                    let len = *len;

                    if *exact {
                        self.match_object(len, |object| object.len() == len)?;
                    } else {
                        self.match_object(len, |object| object.len() >= len)?;
                    }
                }
                Inst::Ptr { offset } => {
                    self.op_ptr(*offset)?;
                }
                Inst::Deref => {
                    self.op_deref()?;
                }
                Inst::Panic { mark } => {
                    return Err(VmError::Panic { mark: *mark });
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
            .field("frame_top", &self.frame_top)
            .field("exited", &self.exited)
            .field("stack", &self.stack)
            .field("frames", &self.frames)
            .field("slots", &DebugSlab(&self.slots))
            .finish()
    }
}

struct DebugSlab<'a, T>(&'a Slab<T>);

impl<T> fmt::Debug for DebugSlab<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_map().entries(self.0.iter()).finish()
    }
}

/// The task of a unit being run.
pub struct Task<'a, T> {
    /// The virtual machine of the task.
    pub vm: &'a mut Vm,
    /// Functions collection associated with the task.
    pub context: &'a Context,
    /// The unit associated with the task.
    pub unit: &'a Unit,
    /// Hold the type of the task.
    _marker: PhantomData<T>,
}

impl<'a, T> Task<'a, T>
where
    T: FromValue,
{
    /// Run the given task to completion.
    pub async fn run_to_completion(self) -> Result<T, VmError> {
        while !self.vm.exited {
            self.vm.run_for(self.context, self.unit, None).await?;
        }

        let value = self.vm.pop_decode()?;
        Ok(value)
    }

    /// Step the given task until the return value is available.
    pub async fn step(&mut self) -> Result<Option<T>, VmError> {
        self.vm.run_for(self.context, self.unit, Some(1)).await?;

        if self.vm.exited {
            let value = self.vm.pop_decode()?;
            return Ok(Some(value));
        }

        Ok(None)
    }
}
