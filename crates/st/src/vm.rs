use crate::external::External;
use crate::functions::{CallError, Functions};
use crate::hash::Hash;
use crate::reflection::{EncodeError, FromValue, IntoArgs};
use crate::unit::Unit;
use crate::value::{
    ExternalTypeError, Managed, Slot, Value, ValueError, ValuePtr, ValueRef, ValueTypeInfo,
};
use anyhow::Result;
use slab::Slab;
use std::any::{type_name, TypeId};
use std::cell::{Cell, RefCell, UnsafeCell};
use std::fmt;
use std::marker::PhantomData;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StackError {
    #[error("stack is empty")]
    StackEmpty,
    #[error("stack frames are empty")]
    StackFramesEmpty,
    #[error("tried to access string at missing slot `{slot}`")]
    StringSlotMissing { slot: usize },
    #[error("tried to access missing array slot `{slot}`")]
    ArraySlotMissing { slot: usize },
    #[error("tried to access missing external slot `{slot}`")]
    ExternalSlotMissing { slot: usize },
}

#[derive(Debug, Error)]
pub enum VmError {
    #[error("failed to encode arguments")]
    EncodeError(#[from] EncodeError),
    #[error("stack error")]
    StackError(#[from] StackError),
    #[error("missing function with hash `{hash}`")]
    MissingFunction { hash: Hash },
    #[error("missing module with hash `{module}`")]
    MissingModule { module: Hash },
    #[error("missing function with hash `{hash}` in module with hash `{module}`")]
    MissingModuleFunction { module: Hash, hash: Hash },
    #[error("error while calling function")]
    CallError(#[from] CallError),
    #[error("instruction pointer is out-of-bounds")]
    IpOutOfBounds,
    #[error("unexpected stack value, expected `{expected}` but was `{actual}`")]
    StackTopTypeError {
        expected: ValueTypeInfo,
        actual: ValueTypeInfo,
    },
    #[error("failed to resolve type info for external type")]
    ExternalTypeError(#[from] ExternalTypeError),
    #[error("unsupported vm operation `{a} {op} {b}`")]
    UnsupportedOperation {
        op: &'static str,
        a: ValueTypeInfo,
        b: ValueTypeInfo,
    },
    #[error("no stack frames to pop")]
    NoStackFrame,
    #[error("tried to access an out-of-bounds stack entry")]
    StackOutOfBounds,
    #[error("tried to access a slot which is out of bounds")]
    SlotOutOfBounds,
    #[error("tried to access an out-of-bounds frame")]
    FrameOutOfBounds,
    #[error("failed to convert value `{actual}`, expected `{expected}`")]
    ConversionError {
        expected: &'static str,
        actual: ValueTypeInfo,
    },
    #[error("static string slot `{slot}` does not exist")]
    MissingStaticString { slot: usize },
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
            (a, b) => return Err(VmError::UnsupportedOperation {
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
            (a, b) => return Err(VmError::UnsupportedOperation {
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
    pub fn iter_stack_debug(&self) -> impl Iterator<Item = (ValuePtr, ValueRef)> + '_ {
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

        args.encode(self)?;

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

        if let Some((managed, slot)) = value.into_managed() {
            self.inc_count(managed, slot)?;
        }

        Ok(())
    }

    /// Pop a value from the stack, freeing it if it's no longer use.
    pub fn managed_pop(&mut self) -> Result<ValuePtr, StackError> {
        let value = self.stack.pop().ok_or_else(|| StackError::StackEmpty)?;

        if let Some((managed, slot)) = value.into_managed() {
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
                            if let Some((managed, slot)) = value.into_managed() {
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

        if let Some((managed, slot)) = value.into_managed() {
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
                    .ok_or_else(|| StackError::ExternalSlotMissing { slot })?;
                holder.count += 1;
            }
            Managed::Array => {
                let holder = self
                    .arrays
                    .get_mut(slot)
                    .ok_or_else(|| StackError::ExternalSlotMissing { slot })?;
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
        match self.strings.get(slot) {
            Some(holder) => Ok(&holder.value),
            None => Err(StackError::StringSlotMissing { slot }),
        }
    }

    /// Get a cloned string from the given slot.
    pub fn string_clone(&self, index: usize) -> Option<Box<str>> {
        Some(self.strings.get(index)?.value.to_owned())
    }

    /// Take the string at the given slot.
    pub fn string_take(&mut self, slot: usize) -> Option<Box<str>> {
        if !self.strings.contains(slot) {
            return None;
        }

        let holder = self.strings.remove(slot);
        Some(holder.value)
    }

    /// Get a cloned array from the given slot.
    pub fn array_clone(&self, index: usize) -> Option<Box<[ValuePtr]>> {
        Some(self.arrays.get(index)?.value.to_owned())
    }

    /// Take the array at the given slot.
    pub fn array_take(&mut self, slot: usize) -> Option<Box<[ValuePtr]>> {
        if !self.arrays.contains(slot) {
            return None;
        }

        let holder = self.arrays.remove(slot);
        Some(holder.value)
    }

    /// Get a clone of the given external.
    pub fn external_clone<T: Clone + External>(&self, slot: usize) -> Option<T> {
        // This is safe since we can rely on the typical reference guarantees of
        // VM.
        unsafe {
            let external = self.externals.get(slot)?;
            let external = (*external.value.get()).as_any().downcast_ref::<T>()?;
            Some(external.clone())
        }
    }

    /// Take an external value by dyn, assuming you have exlusive access to it.
    pub fn external_take<T>(&mut self, slot: usize) -> Option<T>
    where
        T: External,
    {
        if !self.externals.contains(slot) {
            return None;
        }

        let mut external = self.externals.remove(slot);

        // Safety: We have mutable access to the VM, so we're the only ones
        // accessing this right now.
        unsafe {
            let value = Box::into_raw(external.value);

            if let Some(ptr) = (&mut *(*value).get()).as_mut_ptr(TypeId::of::<T>()) {
                return Some(*Box::from_raw(ptr as *mut T));
            }

            external.value = Box::from_raw(value);
            let new_slot = self.externals.insert(external);
            debug_assert!(new_slot == slot);
            None
        }
    }

    /// Take an external value by dyn, assuming you have exlusive access to it.
    pub fn external_take_dyn(&mut self, slot: usize) -> Option<Box<dyn External>> {
        if !self.externals.contains(slot) {
            return None;
        }

        // Safety: We have mutable access to the VM, so we're the only ones
        // accessing this right now.
        unsafe {
            let external = self.externals.remove(slot);
            let value = Box::into_raw(external.value);
            Some(Box::from_raw((*value).get()))
        }
    }

    /// Get a reference of the external value of the given type and the given
    /// slot.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the made up returned reference is no longer used
    /// before [disarm][Vm::disarm] is called.
    pub fn external_dyn_ref(&self, slot: usize) -> Option<&dyn External> {
        let external = self.externals.get(slot)?;

        if !external.access.is_sharable() {
            return None;
        }

        Some(unsafe { &*external.value.get() })
    }

    /// Get a reference of the external value of the given type and the given
    /// slot.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the made up returned reference is no longer used
    /// before [disarm][Vm::disarm] is called.
    pub unsafe fn external_ref<'out, T: External>(&self, slot: usize) -> Option<&'out T> {
        let external = self.externals.get(slot)?;

        if !external.access.shared() {
            return None;
        }

        let external = (&*external.value.get()).as_any().downcast_ref::<T>()?;
        self.guards.borrow_mut().push((Managed::External, slot));
        Some(&*(external as *const T))
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
    pub unsafe fn external_mut<'out, T: External>(&self, slot: usize) -> Option<&'out mut T> {
        let external = self.externals.get(slot)?;

        if !external.access.exclusive() {
            return None;
        }

        let external = (&mut *external.value.get())
            .as_any_mut()
            .downcast_mut::<T>()?;
        self.guards.borrow_mut().push((Managed::External, slot));
        Some(external)
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
    pub fn external_type(&self, index: usize) -> Option<(&'static str, TypeId)> {
        let external = self.externals.get(index)?;
        Some((external.type_name, external.type_id))
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
            Err(e) => {
                let type_info = e.type_info(self)?;

                return Err(VmError::ConversionError {
                    expected: type_name::<T>(),
                    actual: type_info,
                });
            }
        };

        self.gc()?;
        Ok(value)
    }

    /// Convert into an owned array.
    pub fn take_owned_array(&mut self, values: Box<[ValuePtr]>) -> Box<[Value]> {
        let mut output = Vec::with_capacity(values.len());

        for value in values.iter().copied() {
            output.push(self.take_owned_value(value));
        }

        output.into_boxed_slice()
    }

    /// Convert a value reference into an owned value.
    pub fn take_owned_value(&mut self, value: ValuePtr) -> Value {
        match value {
            ValuePtr::Unit => Value::Unit,
            ValuePtr::Integer(integer) => Value::Integer(integer),
            ValuePtr::Float(float) => Value::Float(float),
            ValuePtr::Bool(boolean) => Value::Bool(boolean),
            ValuePtr::Managed(managed) => match managed.into_managed() {
                (Managed::String, slot) => match self.strings.get(slot) {
                    Some(string) => Value::String(string.value.to_owned()),
                    None => Value::Error(ValueError::String(slot)),
                },
                (Managed::Array, slot) => match self.arrays.get(slot) {
                    Some(array) => {
                        let array = array.value.to_owned();
                        Value::Array(self.take_owned_array(array))
                    }
                    None => Value::Error(ValueError::Array(slot)),
                },
                (Managed::External, slot) => match self.external_take_dyn(slot) {
                    Some(external) => Value::External(external),
                    None => Value::Error(ValueError::External(slot)),
                },
            },
        }
    }

    /// Convert into an owned array.
    pub fn to_array<'a>(&'a self, values: &[ValuePtr]) -> Box<[ValueRef<'_>]> {
        let mut output = Vec::with_capacity(values.len());

        for value in values.iter().copied() {
            output.push(self.to_value(value));
        }

        output.into_boxed_slice()
    }

    /// Convert a value reference into an owned value.
    pub fn to_value<'a>(&'a self, value: ValuePtr) -> ValueRef<'a> {
        match value {
            ValuePtr::Unit => ValueRef::Unit,
            ValuePtr::Integer(integer) => ValueRef::Integer(integer),
            ValuePtr::Float(float) => ValueRef::Float(float),
            ValuePtr::Bool(boolean) => ValueRef::Bool(boolean),
            ValuePtr::Managed(managed) => match managed.into_managed() {
                (Managed::String, slot) => match self.strings.get(slot) {
                    Some(string) => ValueRef::String(&string.value),
                    None => ValueRef::Error(ValueError::String(slot)),
                },
                (Managed::Array, slot) => match self.arrays.get(slot) {
                    Some(array) => ValueRef::Array(self.to_array(&array.value)),
                    None => ValueRef::Error(ValueError::Array(slot)),
                },
                (Managed::External, slot) => match self.external_dyn_ref(slot) {
                    Some(external) => ValueRef::External(external),
                    None => ValueRef::Error(ValueError::External(slot)),
                },
            },
        }
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

        Err(VmError::UnsupportedOperation {
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
