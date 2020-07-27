use crate::external::External;
use crate::functions::{CallError, Functions};
use crate::hash::{FnDynamicHash, FnHash, Hash};
use crate::reflection::{EncodeError, FromValue, IntoArgs};
use crate::unit::Unit;
use crate::value::{
    ExternalTypeError, TypeHash, Value, ValueError, ValueRef, ValueType, ValueTypeInfo,
};
use anyhow::Result;
use slab::Slab;
use std::any::type_name;
use std::fmt;
use std::marker::PhantomData;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("failed to encode arguments")]
    EncodeError(#[source] EncodeError),
    #[error("missing function (raw: {raw}, dynamic: {dynamic}, full: {full})")]
    MissingFunction {
        raw: FnHash,
        dynamic: FnDynamicHash,
        full: FnHash,
    },
    #[error("missing dynamic function: {0:?}")]
    MissingDynamicFunction(FnDynamicHash),
    #[error("error while calling function")]
    CallError(#[source] CallError),
    #[error("instruction pointer is out-of-bounds")]
    IpOutOfBounds,
    #[error("tried to perform stack operation on empty stack")]
    StackEmpty,
    #[error("unexpected stack value, expected `{expected}` but was `{actual}`")]
    StackTopTypeError {
        expected: ValueTypeInfo,
        actual: ValueTypeInfo,
    },
    #[error("failed to resolve type info for external type")]
    ExternalTypeError(#[source] ExternalTypeError),
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
    #[error("tried to access value at missing slot `{slot}`")]
    ValueSlotMissing { slot: usize },
    #[error("tried to access string at missing slot `{slot}`")]
    StringSlotMissing { slot: usize },
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

impl From<ExternalTypeError> for VmError {
    fn from(error: ExternalTypeError) -> Self {
        Self::ExternalTypeError(error)
    }
}

impl From<EncodeError> for VmError {
    fn from(error: EncodeError) -> Self {
        Self::EncodeError(error)
    }
}

/// Pop and type check a value off the stack.
macro_rules! pop {
    ($vm:expr) => {
        $vm.managed_pop().ok_or_else(|| VmError::StackEmpty)?
    };

    ($vm:expr, $variant:ident) => {
        match pop!($vm) {
            ValueRef::$variant(b) => b,
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
            (ValueRef::Bool($a), ValueRef::Bool($b)) => $a $op $b,
            (ValueRef::Integer($a), ValueRef::Integer($b)) => $a $op $b,
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
            (ValueRef::Float($a), ValueRef::Float($b)) => ValueRef::Float($a $op $b),
            (ValueRef::Integer($a), ValueRef::Integer($b)) => ValueRef::Integer($a $op $b),
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
    /// Perform a dynamic call.
    ///
    /// It will construct a new stack frame which includes the last `args`
    /// number of entries.
    Call {
        /// The hash of the function to call.
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
        match self {
            Self::Call { hash, args } => {
                let dynamic = FnDynamicHash::of(hash, args);

                match unit.lookup(dynamic) {
                    Some(loc) => {
                        vm.push_frame(*ip, args)?;
                        *ip = loc;
                    }
                    None => {
                        let raw = FnHash::raw(hash);

                        let handler = if let Some(handler) = functions.lookup(raw) {
                            handler
                        } else {
                            // Calculate the call hash from the values on the stack.
                            let full = FnHash::of_dynamic_fallible(
                                dynamic,
                                vm.iter_stack_types().take(args),
                            )?;

                            functions
                                .lookup(full)
                                .ok_or_else(|| VmError::MissingFunction { raw, dynamic, full })?
                        };

                        handler(vm, args).await.map_err(VmError::CallError)?;
                    }
                }
            }
            Self::Return => {
                // NB: unmanaged because we're effectively moving the value.
                let return_value = vm.unmanaged_pop().ok_or_else(|| VmError::StackEmpty)?;

                if let Some(frame) = vm.pop_frame() {
                    *ip = frame.ip;
                }

                vm.exited = vm.frames.is_empty();
                vm.unmanaged_push(return_value);
            }
            Self::ReturnUnit => {
                if let Some(frame) = vm.pop_frame() {
                    *ip = frame.ip;
                }

                vm.exited = vm.frames.is_empty();
                vm.managed_push(ValueRef::Unit);
            }
            Self::Pop => {
                vm.managed_pop();
            }
            Self::Integer { number } => {
                vm.managed_push(ValueRef::Integer(number));
            }
            Self::Float { number } => {
                vm.managed_push(ValueRef::Float(number));
            }
            Self::Copy { offset } => {
                vm.stack_copy_frame(offset)?;
            }
            Self::Unit => {
                vm.managed_push(ValueRef::Unit);
            }
            Self::Jump { offset } => {
                *ip = offset;
            }
            Self::Add => {
                let b = pop!(vm);
                let a = pop!(vm);

                let value = match (a, b) {
                    (ValueRef::Float(a), ValueRef::Float(b)) => ValueRef::Float(a + b),
                    (ValueRef::Integer(a), ValueRef::Integer(b)) => ValueRef::Integer(a + b),
                    (ValueRef::String(a), ValueRef::String(b)) => {
                        let a = vm.string_ref(a)?;
                        let b = vm.string_ref(b)?;
                        let mut string = String::with_capacity(a.len() + b.len());
                        string.push_str(a);
                        string.push_str(b);
                        let slot = vm.allocate_string(string.into_boxed_str());
                        ValueRef::String(slot)
                    }
                    (a, b) => {
                        return Err(VmError::UnsupportedOperation {
                            op: "+",
                            a: a.type_info(vm)?,
                            b: b.type_info(vm)?,
                        })
                    }
                };

                vm.managed_push(value);
            }
            Self::Sub => {
                let b = pop!(vm);
                let a = pop!(vm);
                vm.managed_push(numeric_ops!(vm, a - b));
            }
            Self::Div => {
                let b = pop!(vm);
                let a = pop!(vm);
                vm.managed_push(numeric_ops!(vm, a / b));
            }
            Self::Mul => {
                let b = pop!(vm);
                let a = pop!(vm);
                vm.managed_push(numeric_ops!(vm, a * b));
            }
            Self::Gt => {
                let b = pop!(vm);
                let a = pop!(vm);
                vm.managed_push(ValueRef::Bool(primitive_ops!(vm, a > b)));
            }
            Self::Gte => {
                let b = pop!(vm);
                let a = pop!(vm);
                vm.managed_push(ValueRef::Bool(primitive_ops!(vm, a >= b)));
            }
            Self::Lt => {
                let b = pop!(vm);
                let a = pop!(vm);
                vm.managed_push(ValueRef::Bool(primitive_ops!(vm, a < b)));
            }
            Self::Lte => {
                let b = pop!(vm);
                let a = pop!(vm);
                vm.managed_push(ValueRef::Bool(primitive_ops!(vm, a <= b)));
            }
            Self::Eq => {
                let b = pop!(vm);
                let a = pop!(vm);
                vm.managed_push(ValueRef::Bool(primitive_ops!(vm, a == b)));
            }
            Self::Neq => {
                let b = pop!(vm);
                let a = pop!(vm);
                vm.managed_push(ValueRef::Bool(primitive_ops!(vm, a != b)));
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
                    array.push(vm.stack.pop().ok_or_else(|| VmError::StackEmpty)?);
                }

                let array_slot = vm.allocate_array(array);
                vm.managed_push(ValueRef::Array(array_slot));
            }
            Self::String { slot } => {
                let string = unit
                    .lookup_string(slot)
                    .ok_or_else(|| VmError::MissingStaticString { slot })?;
                // TODO: do something sneaky to only allocate the static string once.
                let string_slot = vm.allocate_string(string.to_owned().into_boxed_str());
                vm.managed_push(ValueRef::String(string_slot));
            }
        }

        vm.gc();
        Ok(())
    }
}

/// A reference to a value in a value slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueSlot(usize);

/// The holde of an external value.
pub struct ExternalHolder<T: ?Sized + External> {
    type_name: &'static str,
    value: T,
}

impl<T> fmt::Debug for ExternalHolder<T>
where
    T: ?Sized + External,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("external")
            .field("type_name", &self.type_name)
            .field("value", &&self.value)
            .finish()
    }
}

/// The holder of a single value.
///
/// Maintains the reference count of the value.
pub struct ValueHolder {
    count: usize,
    pub(crate) value: ValueRef,
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
    pub(crate) stack: Vec<usize>,
    /// Frames relative to the stack.
    pub(crate) frames: Vec<Frame>,
    /// Values which needs to be freed.
    gc_freed: Vec<usize>,
    /// The work list for the gc.
    gc_work: Vec<usize>,
    /// ValueRef slots.
    ///
    /// Values in here might indirectly reference other specializes slots.
    pub(crate) values: Slab<ValueHolder>,
    /// Slots with external values.
    pub(crate) externals: Slab<Box<ExternalHolder<dyn External>>>,
    /// Slots with strings.
    pub(crate) strings: Slab<Box<str>>,
    /// Slots with arrays, which themselves reference values.
    pub(crate) arrays: Slab<Vec<usize>>,
    /// We have exited from the last frame.
    pub(crate) exited: bool,
}

impl Vm {
    /// Construct a new ST virtual machine.
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            frames: Vec::new(),
            gc_freed: Vec::new(),
            gc_work: Vec::new(),
            values: Slab::new(),
            externals: Slab::new(),
            strings: Slab::new(),
            arrays: Slab::new(),
            exited: false,
        }
    }

    /// Iterate over the stack, producing the value associated with each stack
    /// item.
    pub fn iter_stack_debug(&self) -> impl Iterator<Item = (usize, Value)> + '_ {
        let mut it = self.stack.iter().copied();

        std::iter::from_fn(move || {
            let value_slot = it.next()?;

            let value = match self.values.get(value_slot) {
                Some(value) => self.to_owned_value(value.value),
                None => Value::Error(ValueError::Stack(value_slot)),
            };

            Some((value_slot, value))
        })
    }

    /// Iterate over stack types from top to bottom.
    ///
    /// This iterator will not end if the stack ends, instead it will error.
    pub(crate) fn iter_stack_types(&self) -> IterStackTypes<'_> {
        IterStackTypes {
            vm: self,
            index: self.stack.len(),
        }
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
        let hash = Hash::of(name);
        let hash = FnDynamicHash::of(hash, A::count());

        let fn_address = unit
            .lookup(hash)
            .ok_or_else(|| VmError::MissingDynamicFunction(hash))?;

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
    pub fn unmanaged_push(&mut self, ValueSlot(value): ValueSlot) {
        self.stack.push(value);
    }

    /// Pop a reference to a value from the stack.
    ///
    /// The reference count of the value being referenced won't be modified.
    pub fn unmanaged_pop(&mut self) -> Option<ValueSlot> {
        self.stack.pop().map(ValueSlot)
    }

    /// Push a value onto the stack and return its stack index.
    pub fn managed_push(&mut self, value: ValueRef) -> usize {
        if let ValueRef::Unit = value {
            self.stack.push(0);
            return 0;
        }

        let index = self.allocate_value(value);
        self.stack.push(index);
        index
    }

    /// Pop a value from the stack, freeing it if it's no longer use.
    pub fn managed_pop(&mut self) -> Option<ValueRef> {
        let value_slot = self.stack.pop()?;
        self.free(value_slot)
    }

    /// Free the given value_slot.
    fn free(&mut self, value_slot: usize) -> Option<ValueRef> {
        if let Some(holder) = self.values.get_mut(value_slot) {
            debug_assert!(holder.count > 0);
            holder.count = holder.count.saturating_sub(1);

            if holder.count == 0 {
                log::trace!("pushing to freed: {}", value_slot);
                self.gc_freed.push(value_slot);
            }

            Some(holder.value)
        } else {
            None
        }
    }

    /// Collect any garbage accumulated.
    ///
    /// This will invalidate external value references.
    pub fn gc(&mut self) {
        let mut gc_work = std::mem::take(&mut self.gc_work);

        while !self.gc_freed.is_empty() {
            gc_work.append(&mut self.gc_freed);

            for slot in gc_work.drain(..) {
                log::trace!("freeing: {}", slot);

                if !self.values.contains(slot) {
                    log::trace!("trying to free non-existant value: {}", slot);
                    continue;
                }

                let v = self.values.remove(slot);
                debug_assert!(v.count == 0);

                match v.value {
                    ValueRef::External(slot) => {
                        if !self.externals.contains(slot) {
                            log::trace!("trying to free non-existant external: {}", slot);
                            continue;
                        }

                        let _ = self.externals.remove(slot);
                    }
                    ValueRef::String(slot) => {
                        if !self.strings.contains(slot) {
                            log::trace!("trying to free non-existant string: {}", slot);
                            continue;
                        }

                        let _ = self.strings.remove(slot);
                    }
                    ValueRef::Array(slot) => {
                        if !self.arrays.contains(slot) {
                            log::trace!("trying to free non-existant array: {}", slot);
                            continue;
                        }

                        let array = self.arrays.remove(slot);

                        for value_slot in array {
                            self.free(value_slot);
                        }
                    }
                    _ => (),
                }
            }
        }

        // NB: Hand back the work buffer since it's most likely sized
        // appropriately.
        self.gc_work = gc_work;
    }

    /// Copy a reference to the value on the exact slot onto the top of the
    /// stack.
    ///
    /// If the index corresponds to an actual value, it's reference count will
    /// be increased.
    pub fn stack_copy_exact(&mut self, offset: usize) -> Result<(), VmError> {
        let value_slot = match self.stack.get(offset).copied() {
            Some(value) => value,
            None => {
                return Err(VmError::StackOutOfBounds);
            }
        };

        if let Some(value) = self.values.get_mut(value_slot) {
            value.count += 1;
            self.stack.push(value_slot);
        } else {
            return Err(VmError::ValueSlotMissing { slot: value_slot });
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
    pub(crate) fn pop_frame(&mut self) -> Option<Frame> {
        let frame = self.frames.pop()?;

        // Pop all values associated with the call frame.
        while self.stack.len() > frame.offset {
            self.managed_pop();
        }

        Some(frame)
    }

    /// Allocate a new empty variable.
    pub fn allocate_value(&mut self, value: ValueRef) -> usize {
        self.values.insert(ValueHolder { count: 1, value })
    }

    /// Allocate a string and return its slot.
    pub fn allocate_string(&mut self, string: Box<str>) -> usize {
        self.strings.insert(string)
    }

    /// Allocate an array and return its slot.
    pub fn allocate_array(&mut self, array: Vec<usize>) -> usize {
        self.arrays.insert(array)
    }

    /// Allocate and insert an external and return its slot.
    pub fn allocate_external<T: External>(&mut self, value: T) -> usize {
        self.externals.insert(Box::new(ExternalHolder {
            type_name: type_name::<T>(),
            value,
        }))
    }

    /// Get a reference of the string at the given string slot.
    pub fn string_ref(&self, slot: usize) -> Result<&str, VmError> {
        match self.strings.get(slot) {
            Some(string) => Ok(&**string),
            None => Err(VmError::StringSlotMissing { slot }),
        }
    }

    /// Get a cloned string from the given slot.
    pub fn string_clone(&self, index: usize) -> Option<Box<str>> {
        self.strings.get(index).cloned()
    }

    /// Get a cloned instance of the external value of the given type and the
    /// given slot.
    pub fn cloned_external<T: External + Clone>(&self, index: usize) -> Option<T> {
        let external = self.externals.get(index)?;
        external
            .as_ref()
            .value
            .as_any()
            .downcast_ref::<T>()
            .cloned()
    }

    /// Access information about an external type, if available.
    pub fn external_type(&self, index: usize) -> Option<(&'static str, TypeHash)> {
        let external = self.externals.get(index)?;
        let any = external.as_ref().value.as_any();
        Some((external.type_name, TypeHash::new(any.type_id())))
    }

    /// Get the last value on the stack.
    pub fn last(&self) -> Option<ValueRef> {
        let index = *self.stack.last()?;
        self.values.get(index).map(|v| v.value)
    }

    /// Pop the last value on the stack and evaluate it as `T`.
    pub fn pop_decode<T>(&mut self) -> Result<T, VmError>
    where
        T: FromValue,
    {
        let value = self.managed_pop().ok_or_else(|| VmError::StackEmpty)?;

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

        self.gc();
        Ok(value)
    }

    /// Convert into an owned array.
    pub fn to_owned_array(&self, values: &[usize]) -> Vec<Value> {
        let mut output = Vec::with_capacity(values.len());

        for value in values.iter().copied() {
            let value = match self.values.get(value) {
                Some(value) => value.value,
                None => {
                    output.push(Value::Error(ValueError::Value(value)));
                    continue;
                }
            };

            output.push(self.to_owned_value(value));
        }

        output
    }

    /// Convert a value reference into an owned value.
    pub fn to_owned_value(&self, value: ValueRef) -> Value {
        match value {
            ValueRef::Unit => Value::Unit,
            ValueRef::String(slot) => match self.strings.get(slot) {
                Some(string) => Value::String(string.to_owned()),
                None => Value::Error(ValueError::String(slot)),
            },
            ValueRef::Array(slot) => match self.arrays.get(slot) {
                Some(array) => Value::Array(self.to_owned_array(array)),
                None => Value::Error(ValueError::Array(slot)),
            },
            ValueRef::Integer(integer) => Value::Integer(integer),
            ValueRef::Float(float) => Value::Float(float),
            ValueRef::Bool(boolean) => Value::Bool(boolean),
            ValueRef::External(slot) => match self.externals.get(slot) {
                Some(external) => Value::External(external.value.clone_external()),
                None => Value::Error(ValueError::External(slot)),
            },
            ValueRef::Fn(hash) => Value::Fn(hash),
        }
    }
}

impl fmt::Debug for Vm {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Vm")
            .field("stack", &self.stack)
            .field("frames", &self.frames)
            .field("gc_freed", &self.gc_freed)
            .field("values", &DebugSlab(&self.values))
            .field("externals", &DebugSlab(&self.externals))
            .field("strings", &DebugSlab(&self.strings))
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

pub(crate) struct IterStackTypes<'a> {
    vm: &'a Vm,
    index: usize,
}

impl<'a> Iterator for IterStackTypes<'a> {
    type Item = Result<ValueType, VmError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 0 {
            return Some(Err(VmError::StackOutOfBounds));
        }

        self.index -= 1;

        let slot = match self.vm.stack.get(self.index).copied() {
            Some(slot) => slot,
            None => return Some(Err(VmError::StackOutOfBounds)),
        };

        let value = match self.vm.values.get(slot) {
            Some(value) => value,
            None => return Some(Err(VmError::ValueSlotMissing { slot })),
        };

        match value.value.value_type(self.vm) {
            Ok(ty) => Some(Ok(ty)),
            Err(e) => Some(Err(VmError::ExternalTypeError(e))),
        }
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
