use crate::external::External;
use crate::functions::Functions;
use crate::hash::Hash;
use crate::reflection::{FromValue, IntoArgs};
use crate::unit::Unit;
use crate::value::{Managed, Slot, Value, ValuePtr, ValueRef, ValueTypeInfo};
use anyhow::Result;
use slab::Slab;
use std::any::{type_name, TypeId};
use std::cell::{Cell, RefCell, UnsafeCell};
use std::fmt;
use std::marker::PhantomData;
use thiserror::Error;

/// An error raised when interacting with types on the stack.
#[derive(Debug, Error)]
pub enum StackError {
    /// stack is empty
    #[error("stack is empty")]
    StackEmpty,
    /// No stack frames.
    #[error("stack frames are empty")]
    StackFramesEmpty,
    /// The given string slot is missing.
    #[error("tried to access string at missing slot `{slot}`")]
    StringSlotMissing {
        /// The slot that was missing.
        slot: usize,
    },
    /// The given array slot is missing.
    #[error("tried to access missing array slot `{slot}`")]
    ArraySlotMissing {
        /// The slot that was missing.
        slot: usize,
    },
    /// The given external slot is missing.
    #[error("tried to access missing external slot `{slot}`")]
    ExternalSlotMissing {
        /// The slot that was missing.
        slot: usize,
    },
    /// The given external slot is inaccessible.
    #[error("external slot `{slot}` is inaccessible")]
    ExternalInaccessible {
        /// The slot that could not be accessed.
        slot: usize,
    },
    /// Error raised when we expect a specific external type but got another.
    #[error("expected external `{expected}`, but was `{actual}`")]
    ExpectedExternalType {
        /// The type that was expected.
        expected: &'static str,
        /// The type that was found.
        actual: &'static str,
    },
    /// Error raised when we expected a boolean value.
    #[error("expected boolean value")]
    ExpectedBoolean,
    /// Error raised when an integer value was expected.
    #[error("expected integer value")]
    ExpectedInteger,
    /// Error raised when we expected a float value.
    #[error("expected float value")]
    ExpectedFloat,
    /// Error raised when we expected a managed value.
    #[error("expected a managed value")]
    ExpectedManaged,
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
    #[error("unsupported vm operation `{a} {op} {b}`")]
    UnsupportedBinaryOperation {
        /// Operation.
        op: &'static str,
        /// Left-hand side operator.
        a: ValueTypeInfo,
        /// Right-hand side operator.
        b: ValueTypeInfo,
    },
    /// Attempt to access out-of-bounds stack item.
    #[error("tried to access an out-of-bounds stack entry")]
    StackOutOfBounds,
    /// Attempt to access out-of-bounds slot.
    #[error("tried to access a slot which is out of bounds")]
    SlotOutOfBounds,
    /// Attempt to access out-of-bounds frame.
    #[error("tried to access an out-of-bounds frame")]
    FrameOutOfBounds,
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
}

/// Pop and type check a value off the stack.
macro_rules! pop {
    ($vm:expr, $variant:ident) => {
        match $vm.managed_pop()? {
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
            (ValuePtr::Bool($a), ValuePtr::Bool($b)) => $a $op $b,
            (ValuePtr::Integer($a), ValuePtr::Integer($b)) => $a $op $b,
            (a, b) => return Err(VmError::UnsupportedBinaryOperation {
                op: stringify!($op),
                a: a.type_info($vm)?,
                b: b.type_info($vm)?,
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
            (a, b) => return Err(VmError::UnsupportedBinaryOperation {
                op: stringify!($op),
                a: a.type_info($vm)?,
                b: b.type_info($vm)?,
            }),
        }
    }
}

/// An operation in the stack-based virtual machine.
#[derive(Debug, Clone, Copy)]
pub enum Inst {
    /// Add two things together.
    ///
    /// This is the result of an `<a> + <b>` expression.
    Add,
    /// Subtract two things.
    ///
    /// This is the result of an `<a> - <b>` expression.
    Sub,
    /// Divide two things.
    ///
    /// This is the result of an `<a> / <b>` expression.
    Div,
    /// Multiply two things.
    ///
    /// This is the result of an `<a> * <b>` expression.
    Mul,
    /// Perform a function call.
    ///
    /// It will construct a new stack frame which includes the last `args`
    /// number of entries.
    Call {
        /// The hash of the module to call.
        module: Hash,
        /// The hash of the function to call.
        hash: Hash,
        /// The number of arguments expected on the stack for this call.
        args: usize,
    },
    /// Perform a instance function call.
    ///
    /// The instance being called on should be on top of the stack, followed by
    /// `args` number of arguments.
    CallInstance {
        /// The hash of the name of the function to call.
        hash: Hash,
        /// The number of arguments expected on the stack for this call.
        args: usize,
    },
    /// Push a literal integer.
    Integer {
        /// The number to push.
        number: i64,
    },
    /// Push a literal float into a slot.
    Float {
        /// The number to push.
        number: f64,
    },
    /// Pop the value on the stack.
    Pop,
    /// Push a variable from a location `offset` relative to the current call
    /// frame.
    ///
    /// A copy is very cheap. It simply means pushing a reference to the stack
    /// and increasing a reference count.
    Copy {
        /// Offset to copy value from.
        offset: usize,
    },
    /// Push a unit value onto the stack.
    Unit,
    /// Pop the current stack frame and restore the instruction pointer from it.
    ///
    /// The stack frame will be cleared, and the value on the top of the stack
    /// will be left on top of it.
    Return,
    /// Pop the current stack frame and restore the instruction pointer from it.
    ///
    /// The stack frame will be cleared, and a unit value will be pushed to the
    /// top of the stack.
    ReturnUnit,
    /// Compare two values on the stack for lt and push the result as a
    /// boolean on the stack.
    Lt,
    /// Compare two values on the stack for gt and push the result as a
    /// boolean on the stack.
    Gt,
    /// Compare two values on the stack for lte and push the result as a
    /// boolean on the stack.
    Lte,
    /// Compare two values on the stack for gte and push the result as a
    /// boolean on the stack.
    Gte,
    /// Compare two values on the stack for equality and push the result as a
    /// boolean on the stack.
    Eq,
    /// Compare two values on the stack for inequality and push the result as a
    /// boolean on the stack.
    Neq,
    /// Unconditionally to the given offset in the current stack frame.
    Jump {
        /// Offset to jump to.
        offset: usize,
    },
    /// Jump to `offset` if there is a boolean on the stack which is `true`.
    JumpIf {
        /// Offset to jump to.
        offset: usize,
    },
    /// Jump to `offset` if there is a boolean on the stack which is `false`.
    JumpIfNot {
        /// Offset to jump to.
        offset: usize,
    },
    /// Construct a push an array value onto the stack. The number of elements
    /// in the array are determined by `count` and are popped from the stack.
    Array {
        /// The size of the array.
        count: usize,
    },
    /// Load a literal string.
    String {
        /// The static string slot to load the string from.
        slot: usize,
    },
}

impl Inst {
    /// Evaluate the current instruction against the stack.
    async fn eval(
        self,
        ip: &mut usize,
        vm: &mut Vm,
        functions: &Functions,
        unit: &Unit,
    ) -> Result<(), VmError> {
        loop {
            match self {
                Self::Call {
                    module: Hash::GLOBAL_MODULE,
                    hash,
                    args,
                } => {
                    match unit.lookup(hash) {
                        Some(loc) => {
                            vm.push_frame(*ip, args)?;
                            *ip = loc;
                        }
                        None => {
                            let handler = functions
                                .lookup(hash)
                                .ok_or_else(|| VmError::MissingFunction { hash })?;

                            let result = handler(vm, args).await;

                            // Safety: We have exclusive access to the VM and
                            // everything that was borrowed during the call can now
                            // be cleared since it's only used in the handler.
                            unsafe {
                                vm.disarm();
                            }

                            result?;
                        }
                    }
                }
                Self::Call { module, hash, args } => {
                    let m = functions
                        .lookup_module(module)
                        .ok_or_else(|| VmError::MissingModule { module })?;

                    let handler = m
                        .lookup(hash)
                        .ok_or_else(|| VmError::MissingModuleFunction { module, hash })?;

                    let result = handler(vm, args).await;

                    // Safety: We have exclusive access to the VM and
                    // everything that was borrowed during the call can now
                    // be cleared since it's only used in the handler.
                    unsafe {
                        vm.disarm();
                    }

                    result?;
                }
                Self::CallInstance { hash, args } => {
                    let instance = vm.peek()?;
                    let ty = instance.value_type(vm)?;

                    let hash = Hash::instance_fn(ty, hash);

                    match unit.lookup(hash) {
                        Some(loc) => {
                            vm.push_frame(*ip, args)?;
                            *ip = loc;
                        }
                        None => {
                            let handler = functions
                                .lookup(hash)
                                .ok_or_else(|| VmError::MissingFunction { hash })?;

                            let result = handler(vm, args).await;

                            // Safety: We have exclusive access to the VM and
                            // everything that was borrowed during the call can
                            // now be cleared since it's only used in the
                            // handler.
                            unsafe {
                                vm.disarm();
                            }

                            result?;
                        }
                    }
                }
                Self::Return => {
                    // NB: unmanaged because we're effectively moving the value.
                    let return_value = vm.unmanaged_pop()?;
                    let frame = vm.pop_frame()?;
                    *ip = frame.ip;
                    vm.exited = vm.frames.is_empty();
                    vm.unmanaged_push(return_value);
                }
                Self::ReturnUnit => {
                    let frame = vm.pop_frame()?;
                    *ip = frame.ip;

                    vm.exited = vm.frames.is_empty();
                    vm.managed_push(ValuePtr::Unit)?;
                }
                Self::Pop => {
                    vm.managed_pop()?;
                }
                Self::Integer { number } => {
                    vm.managed_push(ValuePtr::Integer(number))?;
                }
                Self::Float { number } => {
                    vm.managed_push(ValuePtr::Float(number))?;
                }
                Self::Copy { offset } => {
                    vm.stack_copy_frame(offset)?;
                }
                Self::Unit => {
                    vm.managed_push(ValuePtr::Unit)?;
                }
                Self::Jump { offset } => {
                    *ip = offset;
                }
                Self::Add => {
                    vm.add()?;
                }
                Self::Sub => {
                    let b = vm.managed_pop()?;
                    let a = vm.managed_pop()?;
                    vm.unmanaged_push(numeric_ops!(vm, a - b));
                }
                Self::Div => {
                    let b = vm.managed_pop()?;
                    let a = vm.managed_pop()?;
                    vm.unmanaged_push(numeric_ops!(vm, a / b));
                }
                Self::Mul => {
                    let b = vm.managed_pop()?;
                    let a = vm.managed_pop()?;
                    vm.unmanaged_push(numeric_ops!(vm, a * b));
                }
                Self::Gt => {
                    let b = vm.managed_pop()?;
                    let a = vm.managed_pop()?;
                    vm.unmanaged_push(ValuePtr::Bool(primitive_ops!(vm, a > b)));
                }
                Self::Gte => {
                    let b = vm.managed_pop()?;
                    let a = vm.managed_pop()?;
                    vm.unmanaged_push(ValuePtr::Bool(primitive_ops!(vm, a >= b)));
                }
                Self::Lt => {
                    let b = vm.managed_pop()?;
                    let a = vm.managed_pop()?;
                    vm.unmanaged_push(ValuePtr::Bool(primitive_ops!(vm, a < b)));
                }
                Self::Lte => {
                    let b = vm.managed_pop()?;
                    let a = vm.managed_pop()?;
                    vm.unmanaged_push(ValuePtr::Bool(primitive_ops!(vm, a <= b)));
                }
                Self::Eq => {
                    let b = vm.managed_pop()?;
                    let a = vm.managed_pop()?;
                    vm.unmanaged_push(ValuePtr::Bool(primitive_ops!(vm, a == b)));
                }
                Self::Neq => {
                    let b = vm.managed_pop()?;
                    let a = vm.managed_pop()?;
                    vm.unmanaged_push(ValuePtr::Bool(primitive_ops!(vm, a != b)));
                }
                Self::JumpIf { offset } => {
                    if pop!(vm, Bool) {
                        *ip = offset;
                    }
                }
                Self::JumpIfNot { offset } => {
                    if !pop!(vm, Bool) {
                        *ip = offset;
                    }
                }
                Self::Array { count } => {
                    let mut array = Vec::with_capacity(count);

                    for _ in 0..count {
                        array.push(vm.stack.pop().ok_or_else(|| StackError::StackEmpty)?);
                    }

                    let value = vm.allocate_array(array.into_boxed_slice());
                    vm.managed_push(value)?;
                }
                Self::String { slot } => {
                    let string = unit
                        .lookup_string(slot)
                        .ok_or_else(|| VmError::MissingStaticString { slot })?;
                    // TODO: do something sneaky to only allocate the static string once.
                    let value = vm.allocate_string(string.to_owned().into_boxed_str());
                    vm.managed_push(value)?;
                }
            }

            break;
        }

        vm.gc()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct Access(Cell<isize>);

impl Access {
    /// Test if the access token is accessible.
    #[inline]
    fn is_sharable(&self) -> bool {
        self.0.get() <= 0
    }

    /// Clear the given access token.
    fn clear(&self) {
        self.0.set(0);
    }

    /// Mark that we want shared access to the given access token.
    #[inline]
    fn shared(&self) -> bool {
        let b = self.0.get().wrapping_sub(1);

        if b < 0 {
            self.0.set(b);
            true
        } else {
            false
        }
    }

    /// Mark that we want exclusive access to the given access token.
    #[inline]
    fn exclusive(&self) -> bool {
        let b = self.0.get().wrapping_add(1);

        if b == 1 {
            self.0.set(b);
            true
        } else {
            false
        }
    }
}

/// The holde of an external value.
pub(crate) struct ExternalHolder<T: ?Sized + External> {
    type_name: &'static str,
    type_id: TypeId,
    count: usize,
    /// How the external is accessed (if it is accessed).
    /// This only happens during function calls, and the function callee is
    /// responsible for unwinding the access.
    access: Access,
    /// The value being held.
    value: Box<UnsafeCell<T>>,
}

impl<T> fmt::Debug for ExternalHolder<T>
where
    T: ?Sized + External,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("external")
            .field("type_name", &self.type_name)
            .field("type_id", &self.type_id)
            .field("count", &self.count)
            .field("access", &self.access)
            .finish()
    }
}

/// The holder of a heap-allocated value.
pub(crate) struct Holder<T> {
    /// Number of references to this value.
    count: usize,
    /// The value being held.
    pub(crate) value: T,
}

impl<T> fmt::Debug for Holder<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(fmt)
    }
}

/// The holder of a single value.
///
/// Maintains the reference count of the value.
pub struct ValueHolder {
    count: usize,
    pub(crate) value: ValuePtr,
}

impl fmt::Debug for ValueHolder {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("value")
            .field("count", &self.count)
            .field("value", &self.value)
            .finish()
    }
}

/// A stack frame.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    /// The stored instruction pointer.
    pub ip: usize,
    /// The stored offset.
    offset: usize,
}

/// A stack which references variables indirectly from a slab.
pub struct Vm {
    /// The current stack of values.
    pub(crate) stack: Vec<ValuePtr>,
    /// Frames relative to the stack.
    pub(crate) frames: Vec<Frame>,
    /// Values which needs to be freed.
    gc_freed: Vec<(Managed, usize)>,
    /// The work list for the gc.
    gc_work: Vec<(Managed, usize)>,
    /// Slots with external values.
    pub(crate) externals: Slab<ExternalHolder<dyn External>>,
    /// Slots with strings.
    pub(crate) strings: Slab<Holder<Box<str>>>,
    /// Slots with arrays, which themselves reference values.
    pub(crate) arrays: Slab<Holder<Box<[ValuePtr]>>>,
    /// We have exited from the last frame.
    pub(crate) exited: bool,
    /// Slots that needs to be disarmed next time we call `disarm`.
    guards: RefCell<Vec<(Managed, usize)>>,
}

impl Vm {
    /// Construct a new ST virtual machine.
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            frames: Vec::new(),
            gc_freed: Vec::new(),
            gc_work: Vec::new(),
            externals: Slab::new(),
            strings: Slab::new(),
            arrays: Slab::new(),
            exited: false,
            guards: RefCell::new(Vec::new()),
        }
    }

    /// Iterate over the stack, producing the value associated with each stack
    /// item.
    pub fn iter_stack_debug(
        &self,
    ) -> impl Iterator<Item = (ValuePtr, Result<ValueRef<'_>, StackError>)> + '_ {
        let mut it = self.stack.iter().copied();

        std::iter::from_fn(move || {
            let value_ref = it.next()?;
            let value = self.to_value(value_ref);
            Some((value_ref, value))
        })
    }

    /// Call the given function in the given compilation unit.
    pub fn call_function<'a, A, T>(
        &'a mut self,
        functions: &'a Functions,
        unit: &'a Unit,
        name: &str,
        args: A,
    ) -> Result<Task<'a, T>, VmError>
    where
        A: IntoArgs,
        T: FromValue,
    {
        let hash = Hash::global_fn(name);

        let fn_address = unit
            .lookup(hash)
            .ok_or_else(|| VmError::MissingFunction { hash })?;

        args.into_args(self)?;

        let offset = self
            .stack
            .len()
            .checked_sub(A::count())
            .ok_or_else(|| VmError::StackOutOfBounds)?;

        self.frames.push(Frame { ip: 0, offset });

        Ok(Task {
            vm: self,
            ip: fn_address,
            functions,
            unit,
            _marker: PhantomData,
        })
    }

    /// Run the given program on the virtual machine.
    pub fn run<'a, T>(&'a mut self, functions: &'a Functions, unit: &'a Unit) -> Task<'a, T>
    where
        T: FromValue,
    {
        Task {
            vm: self,
            ip: 0,
            functions,
            unit,
            _marker: PhantomData,
        }
    }

    /// Push an unmanaged reference.
    ///
    /// The reference count of the value being referenced won't be modified.
    pub fn unmanaged_push(&mut self, value: ValuePtr) {
        self.stack.push(value);
    }

    /// Pop a reference to a value from the stack.
    ///
    /// The reference count of the value being referenced won't be modified.
    pub fn unmanaged_pop(&mut self) -> Result<ValuePtr, StackError> {
        self.stack.pop().ok_or_else(|| StackError::StackEmpty)
    }

    /// Push a value onto the stack and return its stack index.
    pub fn managed_push(&mut self, value: ValuePtr) -> Result<(), StackError> {
        self.stack.push(value);

        if let Some((managed, slot)) = value.try_into_managed() {
            self.inc_count(managed, slot)?;
        }

        Ok(())
    }

    /// Pop a value from the stack, freeing it if it's no longer use.
    pub fn managed_pop(&mut self) -> Result<ValuePtr, StackError> {
        let value = self.stack.pop().ok_or_else(|| StackError::StackEmpty)?;

        if let Some((managed, slot)) = value.try_into_managed() {
            self.dec_count(managed, slot)?;
        }

        Ok(value)
    }

    /// Peek the top of the stack.
    pub fn peek(&mut self) -> Result<ValuePtr, StackError> {
        self.stack
            .last()
            .copied()
            .ok_or_else(|| StackError::StackEmpty)
    }

    /// Collect any garbage accumulated.
    ///
    /// This will invalidate external value references.
    pub fn gc(&mut self) -> Result<(), StackError> {
        let mut gc_work = std::mem::take(&mut self.gc_work);

        while !self.gc_freed.is_empty() {
            gc_work.append(&mut self.gc_freed);

            for (managed, slot) in gc_work.drain(..) {
                log::trace!("freeing: {:?}({})", managed, slot);

                match managed {
                    Managed::External => {
                        if !self.externals.contains(slot) {
                            log::trace!("trying to free non-existant external: {}", slot);
                            continue;
                        }

                        let external = self.externals.remove(slot);
                        log::trace!("external freed: {:?}", external);
                        debug_assert!(external.count == 0);
                    }
                    Managed::String => {
                        if !self.strings.contains(slot) {
                            log::trace!("trying to free non-existant string: {}", slot);
                            continue;
                        }

                        let string = self.strings.remove(slot);
                        debug_assert!(string.count == 0);
                    }
                    Managed::Array => {
                        if !self.arrays.contains(slot) {
                            log::trace!("trying to free non-existant array: {}", slot);
                            continue;
                        }

                        let array = self.arrays.remove(slot);

                        for value in array.value.into_iter().copied() {
                            if let Some((managed, slot)) = value.try_into_managed() {
                                self.dec_count(managed, slot)?;
                            }
                        }

                        debug_assert!(array.count == 0);
                    }
                }
            }
        }

        // NB: Hand back the work buffer since it's most likely sized
        // appropriately.
        self.gc_work = gc_work;
        Ok(())
    }

    /// Copy a reference to the value on the exact slot onto the top of the
    /// stack.
    ///
    /// If the index corresponds to an actual value, it's reference count will
    /// be increased.
    pub fn stack_copy_exact(&mut self, offset: usize) -> Result<(), VmError> {
        let value = match self.stack.get(offset).copied() {
            Some(value) => value,
            None => {
                return Err(VmError::StackOutOfBounds);
            }
        };

        if let Some((managed, slot)) = value.try_into_managed() {
            self.inc_count(managed, slot)?;
        }

        self.stack.push(value);
        Ok(())
    }

    /// Decrement reference count of value reference.
    fn inc_count(&mut self, managed: Managed, slot: usize) -> Result<(), StackError> {
        match managed {
            Managed::String => {
                let holder = self
                    .strings
                    .get_mut(slot)
                    .ok_or_else(|| StackError::StringSlotMissing { slot })?;
                holder.count += 1;
            }
            Managed::Array => {
                let holder = self
                    .arrays
                    .get_mut(slot)
                    .ok_or_else(|| StackError::ArraySlotMissing { slot })?;
                holder.count += 1;
            }
            Managed::External => {
                let holder = self
                    .externals
                    .get_mut(slot)
                    .ok_or_else(|| StackError::ExternalSlotMissing { slot })?;
                holder.count += 1;
            }
        }

        Ok(())
    }

    /// Decrement ref count and free if appropriate.
    fn dec_count(&mut self, managed: Managed, slot: usize) -> Result<(), StackError> {
        match managed {
            Managed::String => {
                let holder = match self.strings.get_mut(slot) {
                    Some(holder) => holder,
                    None => return Ok(()),
                };

                debug_assert!(holder.count > 0);
                holder.count = holder.count.saturating_sub(1);

                if holder.count == 0 {
                    log::trace!("pushing to freed: {:?}({})", managed, slot);
                    self.gc_freed.push((managed, slot));
                }
            }
            Managed::Array => {
                let holder = match self.arrays.get_mut(slot) {
                    Some(holder) => holder,
                    None => return Ok(()),
                };

                debug_assert!(holder.count > 0);
                holder.count = holder.count.saturating_sub(1);

                if holder.count == 0 {
                    log::trace!("pushing to freed: {:?}({})", managed, slot);
                    self.gc_freed.push((managed, slot));
                }
            }
            Managed::External => {
                // NB: has been moved externally.
                let holder = match self.externals.get_mut(slot) {
                    Some(holder) => holder,
                    None => return Ok(()),
                };

                debug_assert!(holder.count > 0);
                holder.count = holder.count.saturating_sub(1);

                if holder.count == 0 {
                    log::trace!("pushing to freed: {:?}({})", managed, slot);
                    self.gc_freed.push((managed, slot));
                }
            }
        }

        Ok(())
    }

    /// Copy a single location from the stack and push it onto the stack
    /// relative to the current stack frame.
    ///
    /// If the index corresponds to an actual value, it's reference count will
    /// be increased.
    pub fn stack_copy_frame(&mut self, rel: usize) -> Result<(), VmError> {
        let slot = if let Some(Frame { offset, .. }) = self.frames.last().copied() {
            offset
                .checked_add(rel)
                .ok_or_else(|| VmError::SlotOutOfBounds)?
        } else {
            rel
        };

        self.stack_copy_exact(slot)
    }

    /// Push a new call frame.
    pub(crate) fn push_frame(&mut self, ip: usize, args: usize) -> Result<(), VmError> {
        let offset = self
            .stack
            .len()
            .checked_sub(args)
            .ok_or_else(|| VmError::StackOutOfBounds)?;

        self.frames.push(Frame { ip, offset });
        Ok(())
    }

    /// Pop a call frame and return it.
    pub(crate) fn pop_frame(&mut self) -> Result<Frame, StackError> {
        let frame = self
            .frames
            .pop()
            .ok_or_else(|| StackError::StackFramesEmpty)?;

        // Pop all values associated with the call frame.
        while self.stack.len() > frame.offset {
            self.managed_pop()?;
        }

        Ok(frame)
    }

    /// Allocate a string and return its value reference.
    ///
    /// This operation can leak memory unless the returned slot is pushed onto
    /// the stack.
    pub fn allocate_string(&mut self, string: Box<str>) -> ValuePtr {
        let slot = self.strings.insert(Holder {
            count: 0,
            value: string,
        });

        ValuePtr::Managed(Slot::string(slot))
    }

    /// Allocate an array and return its value reference.
    ///
    /// This operation can leak memory unless the returned slot is pushed onto
    /// the stack.
    pub fn allocate_array(&mut self, array: Box<[ValuePtr]>) -> ValuePtr {
        let slot = self.arrays.insert(Holder {
            count: 0,
            value: array,
        });

        ValuePtr::Managed(Slot::array(slot))
    }

    /// Allocate and insert an external and return its reference.
    ///
    /// This will leak memory unless the reference is pushed onto the stack to
    /// be managed.
    pub fn allocate_external<T: External>(&mut self, value: T) -> ValuePtr {
        let slot = self.externals.insert(ExternalHolder {
            type_name: type_name::<T>(),
            type_id: TypeId::of::<T>(),
            count: 0,
            access: Access::default(),
            value: Box::new(UnsafeCell::new(value)),
        });

        ValuePtr::Managed(Slot::external(slot))
    }

    /// Get a reference of the string at the given string slot.
    pub fn string_ref(&self, slot: usize) -> Result<&str, StackError> {
        if let Some(holder) = self.strings.get(slot) {
            return Ok(&holder.value);
        }

        Err(StackError::StringSlotMissing { slot })
    }

    /// Get a cloned string from the given slot.
    pub fn string_clone(&self, slot: usize) -> Result<Box<str>, StackError> {
        if let Some(holder) = self.strings.get(slot) {
            return Ok(holder.value.to_owned());
        }

        Err(StackError::StringSlotMissing { slot })
    }

    /// Take the string at the given slot.
    pub fn string_take(&mut self, slot: usize) -> Result<Box<str>, StackError> {
        if !self.strings.contains(slot) {
            return Err(StackError::StringSlotMissing { slot });
        }

        let holder = self.strings.remove(slot);
        Ok(holder.value)
    }

    /// Get a reference of the array at the given slot.
    pub fn array_ref(&self, slot: usize) -> Result<&[ValuePtr], StackError> {
        if let Some(holder) = self.arrays.get(slot) {
            return Ok(&holder.value);
        }

        Err(StackError::ArraySlotMissing { slot })
    }

    /// Get a cloned array from the given slot.
    pub fn array_clone(&self, slot: usize) -> Result<Box<[ValuePtr]>, StackError> {
        if let Some(holder) = self.arrays.get(slot) {
            return Ok(holder.value.to_owned());
        }

        Err(StackError::ArraySlotMissing { slot })
    }

    /// Take the array at the given slot.
    pub fn array_take(&mut self, slot: usize) -> Result<Box<[ValuePtr]>, StackError> {
        if !self.arrays.contains(slot) {
            return Err(StackError::ArraySlotMissing { slot });
        }

        let holder = self.arrays.remove(slot);
        Ok(holder.value)
    }

    /// Get a clone of the given external.
    pub fn external_clone<T: Clone + External>(&self, slot: usize) -> Result<T, StackError> {
        // This is safe since we can rely on the typical reference guarantees of
        // VM.
        unsafe {
            if let Some(holder) = self.externals.get(slot) {
                let external = (*holder.value.get())
                    .as_any()
                    .downcast_ref::<T>()
                    .ok_or_else(|| StackError::ExpectedExternalType {
                        expected: type_name::<T>(),
                        actual: holder.type_name,
                    })?;

                return Ok(external.clone());
            }

            Err(StackError::ExternalSlotMissing { slot })
        }
    }

    /// Take an external value by dyn, assuming you have exlusive access to it.
    pub fn external_take<T>(&mut self, slot: usize) -> Result<T, StackError>
    where
        T: External,
    {
        if !self.externals.contains(slot) {
            return Err(StackError::ExternalSlotMissing { slot });
        }

        let mut external = self.externals.remove(slot);

        // Safety: We have mutable access to the VM, so we're the only ones
        // accessing this right now.
        unsafe {
            let value = Box::into_raw(external.value);

            if let Some(ptr) = (&mut *(*value).get()).as_mut_ptr(TypeId::of::<T>()) {
                return Ok(*Box::from_raw(ptr as *mut T));
            }

            let actual = external.type_name;

            external.value = Box::from_raw(value);
            let new_slot = self.externals.insert(external);
            debug_assert!(new_slot == slot);

            Err(StackError::ExpectedExternalType {
                expected: type_name::<T>(),
                actual,
            })
        }
    }

    /// Take an external value by dyn, assuming you have exlusive access to it.
    pub fn external_take_dyn(&mut self, slot: usize) -> Result<Box<dyn External>, StackError> {
        if !self.externals.contains(slot) {
            return Err(StackError::ExternalSlotMissing { slot });
        }

        // Safety: We have mutable access to the VM, so we're the only ones
        // accessing this right now.
        unsafe {
            let external = self.externals.remove(slot);
            let value = Box::into_raw(external.value);
            Ok(Box::from_raw((*value).get()))
        }
    }

    /// Get a reference of the external value of the given type and the given
    /// slot.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the made up returned reference is no longer used
    /// before [disarm][Vm::disarm] is called.
    pub fn external_ref_dyn(&self, slot: usize) -> Result<&dyn External, StackError> {
        let external = self
            .externals
            .get(slot)
            .ok_or_else(|| StackError::ExternalSlotMissing { slot })?;

        if !external.access.is_sharable() {
            return Err(StackError::ExternalInaccessible { slot });
        }

        Ok(unsafe { &*external.value.get() })
    }

    /// Get a reference of the external value of the given type and the given
    /// slot.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the made up returned reference is no longer used
    /// before [disarm][Vm::disarm] is called.
    pub unsafe fn unsafe_external_ref<'out, T: External>(
        &self,
        slot: usize,
    ) -> Result<&'out T, StackError> {
        let external = self
            .externals
            .get(slot)
            .ok_or_else(|| StackError::ExternalSlotMissing { slot })?;

        if !external.access.shared() {
            return Err(StackError::ExternalInaccessible { slot });
        }

        let external = (&*external.value.get())
            .as_any()
            .downcast_ref::<T>()
            .ok_or_else(|| StackError::ExpectedExternalType {
                expected: type_name::<T>(),
                actual: external.type_name,
            })?;

        self.guards.borrow_mut().push((Managed::External, slot));
        Ok(&*(external as *const T))
    }

    /// Get a reference of the external value of the given type and the given
    /// slot.
    ///
    /// Mark the given value as mutably used, preventing it from being used
    /// again.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the made up returned reference is no longer used
    /// before [disarm][Vm::disarm] is called.
    pub unsafe fn unsafe_external_mut<'out, T: External>(
        &self,
        slot: usize,
    ) -> Result<&'out mut T, StackError> {
        let external = self
            .externals
            .get(slot)
            .ok_or_else(|| StackError::ExternalSlotMissing { slot })?;

        if !external.access.exclusive() {
            return Err(StackError::ExternalInaccessible { slot });
        }

        let external = (&mut *external.value.get())
            .as_any_mut()
            .downcast_mut::<T>()
            .ok_or_else(|| StackError::ExpectedExternalType {
                expected: type_name::<T>(),
                actual: external.type_name,
            })?;

        self.guards.borrow_mut().push((Managed::External, slot));
        Ok(external)
    }

    /// Disarm all collected guards.
    pub unsafe fn disarm(&self) {
        for (managed, slot) in self.guards.borrow_mut().drain(..) {
            match managed {
                Managed::External => {
                    if let Some(holder) = self.externals.get(slot) {
                        holder.access.clear();
                    }
                }
                _ => (),
            }
        }
    }

    /// Access information about an external type, if available.
    pub fn external_type(&self, slot: usize) -> Result<(&'static str, TypeId), StackError> {
        if let Some(holder) = self.externals.get(slot) {
            return Ok((holder.type_name, holder.type_id));
        }

        Err(StackError::ExternalSlotMissing { slot })
    }

    /// Get the last value on the stack.
    pub fn last(&self) -> Option<ValuePtr> {
        self.stack.last().copied()
    }

    /// Pop the last value on the stack and evaluate it as `T`.
    pub fn pop_decode<T>(&mut self) -> Result<T, VmError>
    where
        T: FromValue,
    {
        let value = self.unmanaged_pop()?;

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

        self.gc()?;
        Ok(value)
    }

    /// Convert into an owned array.
    pub fn take_owned_array(
        &mut self,
        values: Box<[ValuePtr]>,
    ) -> Result<Box<[Value]>, StackError> {
        let mut output = Vec::with_capacity(values.len());

        for value in values.iter().copied() {
            output.push(self.take_owned_value(value)?);
        }

        Ok(output.into_boxed_slice())
    }

    /// Convert a value reference into an owned value.
    pub fn take_owned_value(&mut self, value: ValuePtr) -> Result<Value, StackError> {
        Ok(match value {
            ValuePtr::Unit => Value::Unit,
            ValuePtr::Integer(integer) => Value::Integer(integer),
            ValuePtr::Float(float) => Value::Float(float),
            ValuePtr::Bool(boolean) => Value::Bool(boolean),
            ValuePtr::Managed(managed) => match managed.into_managed() {
                (Managed::String, slot) => Value::String(self.string_take(slot)?),
                (Managed::Array, slot) => {
                    let array = self.array_take(slot)?;
                    Value::Array(self.take_owned_array(array)?)
                }
                (Managed::External, slot) => Value::External(self.external_take_dyn(slot)?),
            },
        })
    }

    /// Convert into an owned array.
    pub fn to_array<'a>(&'a self, values: &[ValuePtr]) -> Result<Box<[ValueRef<'_>]>, StackError> {
        let mut output = Vec::with_capacity(values.len());

        for value in values.iter().copied() {
            output.push(self.to_value(value)?);
        }

        Ok(output.into_boxed_slice())
    }

    /// Convert a value reference into an owned value.
    pub fn to_value<'a>(&'a self, value: ValuePtr) -> Result<ValueRef<'a>, StackError> {
        Ok(match value {
            ValuePtr::Unit => ValueRef::Unit,
            ValuePtr::Integer(integer) => ValueRef::Integer(integer),
            ValuePtr::Float(float) => ValueRef::Float(float),
            ValuePtr::Bool(boolean) => ValueRef::Bool(boolean),
            ValuePtr::Managed(managed) => match managed.into_managed() {
                (Managed::String, slot) => ValueRef::String(self.string_ref(slot)?),
                (Managed::Array, slot) => {
                    let array = self.array_ref(slot)?;
                    ValueRef::Array(self.to_array(array)?)
                }
                (Managed::External, slot) => ValueRef::External(self.external_ref_dyn(slot)?),
            },
        })
    }

    /// Implementation of the add operation.
    fn add(&mut self) -> Result<(), VmError> {
        let b = self.managed_pop()?;
        let a = self.managed_pop()?;

        match (a, b) {
            (ValuePtr::Float(a), ValuePtr::Float(b)) => {
                self.managed_push(ValuePtr::Float(a + b))?;
                return Ok(());
            }
            (ValuePtr::Integer(a), ValuePtr::Integer(b)) => {
                self.managed_push(ValuePtr::Integer(a + b))?;
                return Ok(());
            }
            (ValuePtr::Managed(a), ValuePtr::Managed(b)) => {
                match (a.into_managed(), b.into_managed()) {
                    ((Managed::String, a), (Managed::String, b)) => {
                        let a = self.string_ref(a)?;
                        let b = self.string_ref(b)?;
                        let mut string = String::with_capacity(a.len() + b.len());
                        string.push_str(a);
                        string.push_str(b);
                        let value = self.allocate_string(string.into_boxed_str());
                        self.managed_push(value)?;
                        return Ok(());
                    }
                    _ => (),
                }
            }
            _ => (),
        };

        Err(VmError::UnsupportedBinaryOperation {
            op: "+",
            a: a.type_info(self)?,
            b: b.type_info(self)?,
        })
    }
}

impl fmt::Debug for Vm {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Vm")
            .field("stack", &self.stack)
            .field("frames", &self.frames)
            .field("gc_freed", &self.gc_freed)
            .field("externals", &DebugSlab(&self.externals))
            .field("strings", &DebugSlab(&self.strings))
            .field("arrays", &DebugSlab(&self.arrays))
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
    /// The instruction pointer of the task.
    pub ip: usize,
    /// Functions collection associated with the task.
    pub functions: &'a Functions,
    /// The unit associated with the task.
    pub unit: &'a Unit,
    _marker: PhantomData<T>,
}

impl<'a, T> Task<'a, T>
where
    T: FromValue,
{
    /// Run the given task to completion.
    pub async fn run_to_completion(mut self) -> Result<T, VmError> {
        while !self.vm.exited {
            let inst = self
                .unit
                .instruction_at(self.ip)
                .ok_or_else(|| VmError::IpOutOfBounds)?;

            self.ip += 1;
            inst.eval(&mut self.ip, &mut self.vm, self.functions, self.unit)
                .await?;
        }

        Ok(self.vm.pop_decode()?)
    }

    /// Step the given task until the return value is available.
    pub async fn step(&mut self) -> Result<Option<T>, VmError> {
        let inst = self
            .unit
            .instruction_at(self.ip)
            .ok_or_else(|| VmError::IpOutOfBounds)?;

        self.ip += 1;
        inst.eval(&mut self.ip, &mut self.vm, self.functions, self.unit)
            .await?;

        if self.vm.exited {
            return Ok(Some(self.vm.pop_decode()?));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::Access;

    #[test]
    fn test_access() {
        let access = Access::default();

        assert!(access.is_sharable());
        assert!(access.shared());
        assert!(!access.exclusive());
        assert!(access.shared());
        access.clear();
        assert!(access.exclusive());
        assert!(!access.exclusive());
        access.clear();
        assert!(access.exclusive());
    }
}
