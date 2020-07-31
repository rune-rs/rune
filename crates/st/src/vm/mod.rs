use crate::collections::HashMap;
use crate::external::External;
use crate::functions::Functions;
use crate::hash::Hash;
use crate::reflection::{FromValue, IntoArgs};
use crate::unit::Unit;
use crate::value::{Managed, Slot, Value, ValuePtr, ValueRef, ValueTypeInfo};
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

use self::access::{Access, Guard};
pub use self::access::{Mut, Ref};
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
    /// The given object slot is missing.
    #[error("tried to access missing object slot `{slot}`")]
    ObjectSlotMissing {
        /// The slot that was missing.
        slot: usize,
    },
    /// The given external slot is missing.
    #[error("tried to access missing external slot `{slot}`")]
    ExternalSlotMissing {
        /// The slot that was missing.
        slot: usize,
    },
    /// The given slot is inaccessible.
    #[error("{managed} slot `{slot}` is inaccessible for exclusive access")]
    SlotInaccessibleExclusive {
        /// Error raised when a slot is inaccessible.
        managed: Managed,
        /// The slot that could not be accessed.
        slot: usize,
    },
    /// The given slot is inaccessible.
    #[error("{managed} slot `{slot}` is inaccessible for shared access")]
    SlotInaccessibleShared {
        /// Error raised when a slot is inaccessible.
        managed: Managed,
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
    /// Error raised when we expected a char value.
    #[error("expected char value")]
    ExpectedChar,
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
    /// An is operation is not supported.
    #[error("`{value_type} is {test_type}` is not supported")]
    UnsupportedIs {
        /// The argument that is not supported.
        value_type: ValueTypeInfo,
        /// The type that is not supported.
        test_type: ValueTypeInfo,
    },
    /// Missing type.
    #[error("no type matching hash `{hash}`")]
    MissingType {
        /// Hash of the type missing.
        hash: Hash,
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
struct ExternalHolder<T: ?Sized + External> {
    /// The name of the stored value.
    type_name: &'static str,
    /// The type id of the stored value.
    type_id: TypeId,
    /// The number of things referencing this external.
    count: usize,
    /// How the external is accessed (if it is accessed).
    /// This only happens during function calls, and the function callee is
    /// responsible for unwinding the access.
    access: Access,
    /// The value being held.
    value: Option<Box<UnsafeCell<T>>>,
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
struct Holder<T> {
    /// Number of references to this value.
    count: usize,
    /// Access for the given holder.
    access: Access,
    /// The value being held.
    value: Option<UnsafeCell<T>>,
}

impl<T> fmt::Debug for Holder<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(fmt)
    }
}

/// A stack frame.
#[derive(Debug, Clone, Copy)]
struct Frame {
    /// The stored instruction pointer.
    ip: usize,
    /// The stored offset.
    frame_top: usize,
}

macro_rules! impl_ref_count {
    (
        $({$managed:ident, $field:ident, $error:ident},)*
    ) => {
        /// Decrement reference count of value reference.
        fn inc_ref(&mut self, managed: Managed, slot: usize) -> Result<(), StackError> {
            match managed {
                $(Managed::$managed => {
                    let holder = self
                        .$field
                        .get_mut(slot)
                        .ok_or_else(|| StackError::$error { slot })?;
                    holder.count += 1;
                })*
            }

            Ok(())
        }

        /// Decrement ref count and free if appropriate.
        fn dec_ref(&mut self, managed: Managed, slot: usize) -> Result<(), StackError> {
            match managed {
                $(Managed::$managed => {
                    let holder = match self.$field.get_mut(slot) {
                        Some(holder) => holder,
                        None => return Ok(()),
                    };

                    debug_assert!(holder.count > 0);
                    holder.count = holder.count.saturating_sub(1);

                    if holder.count == 0 {
                        log::trace!("pushing to freed: {:?}({})", managed, slot);
                        self.reap_queue.push((managed, slot));
                    }
                })*
            }

            Ok(())
        }

        /// Disarm all collected guards.
        ///
        /// Borrows are "armed" if they are unsafely converted into an inner unbound
        /// reference through either [unsafe_into_ref][Ref::unsafe_into_ref] or
        /// [unsafe_into_mut][Mut::unsafe_into_mut].
        ///
        /// After this happens, the slot is permanently marked as used (either
        /// exclusively or shared) until this disarm function is called.
        ///
        /// However, the caller of this function **must** provide some safety
        /// guarantees documented below.
        ///
        /// # Safety
        ///
        /// This may only be called once all references fetched through the various
        /// `*_mut` and `*_ref` methods are no longer live.
        ///
        /// Otherwise, this could permit aliasing of slot values.
        pub unsafe fn disarm(&mut self) {
            // Safety: We have exclusive access to the guards field.
            for (managed, slot) in (*self.guards.get()).drain(..) {
                log::trace!("clearing access: {}({})", managed, slot);

                match managed {
                    $(Managed::$managed => {
                        if let Some(holder) = self.$field.get(slot) {
                            holder.access.clear();
                        }
                    })*
                }
            }
        }
    }
}

macro_rules! impl_slot_functions {
    (
        $field:ident,
        $ret_ty:ty,
        $error:ident,
        $managed:ident,
        $slot:ident,
        $fn_allocate:ident,
        $fn_ref:ident,
        $fn_clone:ident,
        $fn_mut:ident,
        $fn_take:ident,
    ) => {
        /// Allocate a value and return its ptr.
        ///
        /// This operation can leak memory unless the returned slot is pushed onto
        /// the stack.
        ///
        /// Newly allocated items already have a refcount of 1. And should be
        /// pushed on the stack using [unmanaged_push], rather than
        /// [managed_push].
        pub fn $fn_allocate(&mut self, value: $ret_ty) -> ValuePtr {
            let slot = self.$field.insert(Holder {
                count: 1,
                access: Default::default(),
                value: Some(UnsafeCell::new(value)),
            });

            ValuePtr::Managed(Slot::$slot(slot))
        }

        /// Get a reference of the value at the given slot.
        pub fn $fn_ref(&self, slot: usize) -> Result<Ref<'_, $ret_ty>, StackError> {
            let holder = match self.$field.get(slot) {
                Some(holder) => holder,
                None => {
                    return Err(StackError::$error { slot });
                }
            };

            holder.access.shared(Managed::$managed, slot)?;

            let value = match &holder.value {
                Some(value) => value,
                None => {
                    holder.access.release_shared();
                    return Err(StackError::$error { slot });
                }
            };

            // Safety: we wrap the value in the necessary guard to make it safe.
            let value = unsafe { &*value.get() };

            Ok(Ref {
                value,
                access: &holder.access,
                guard: (Managed::$managed, slot),
                guards: &self.guards,
            })
        }

        /// Get a cloned value from the given slot.
        pub fn $fn_clone(&self, slot: usize) -> Result<$ret_ty, StackError> {
            let holder = match self.$field.get(slot) {
                Some(holder) => holder,
                None => {
                    return Err(StackError::$error { slot });
                }
            };

            // NB: we only need temporary access.
            holder.access.test_shared(Managed::$managed, slot)?;

            let value = match &holder.value {
                Some(value) => value,
                None => {
                    return Err(StackError::$error { slot });
                }
            };

            // Safety: Caller needs to ensure that they safely call disarm.
            Ok(unsafe { (*value.get()).clone() })
        }

        /// Get a reference of the value at the given slot.
        pub fn $fn_mut(&self, slot: usize) -> Result<Mut<'_, $ret_ty>, StackError> {
            let holder = match self.$field.get(slot) {
                Some(holder) => holder,
                None => {
                    return Err(StackError::$error { slot });
                }
            };

            holder.access.exclusive(Managed::$managed, slot)?;

            let value = match &holder.value {
                Some(value) => value,
                None => {
                    holder.access.release_exclusive();
                    return Err(StackError::$error { slot });
                }
            };

            // Safety: Caller needs to ensure that they safely call disarm.
            let value = unsafe { &mut *value.get() };

            Ok(Mut {
                value,
                access: &holder.access,
                guard: (Managed::$managed, slot),
                guards: &self.guards,
            })
        }

        /// Take the value at the given slot.
        ///
        /// After taking the value, the caller is responsible for deallocating it.
        pub fn $fn_take(&mut self, slot: usize) -> Result<$ret_ty, StackError> {
            let holder = match self.$field.get_mut(slot) {
                Some(holder) => holder,
                None => {
                    return Err(StackError::$error { slot });
                }
            };

            holder.access.exclusive(Managed::$managed, slot)?;

            let value = match holder.value.take() {
                Some(value) => value,
                None => {
                    holder.access.release_exclusive();
                    return Err(StackError::$error { slot });
                }
            };

            holder.access.release_exclusive();
            Ok(UnsafeCell::into_inner(value))
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
    /// Values which needs to be freed.
    reap_queue: Vec<(Managed, usize)>,
    /// The work list for the reap.
    reap_work: Vec<(Managed, usize)>,
    /// Slots with external values.
    externals: Slab<ExternalHolder<dyn External>>,
    /// Slots with strings.
    strings: Slab<Holder<String>>,
    /// Slots with arrays, which themselves reference values.
    arrays: Slab<Holder<Vec<ValuePtr>>>,
    /// Slots with objects, which themselves reference values.
    objects: Slab<Holder<HashMap<String, ValuePtr>>>,
    /// Slots that needs to be disarmed next time we call `disarm`.
    guards: UnsafeCell<Vec<Guard>>,
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
            reap_queue: Vec::new(),
            reap_work: Vec::new(),
            externals: Slab::new(),
            strings: Slab::new(),
            arrays: Slab::new(),
            objects: Slab::new(),
            guards: UnsafeCell::new(Vec::new()),
        }
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
        functions: &'a Functions,
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

        let fn_address = unit
            .lookup(hash)
            .ok_or_else(|| VmError::MissingFunction { hash })?;

        args.into_args(self)?;

        self.ip = fn_address;
        self.frame_top = 0;

        Ok(Task {
            vm: self,
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
        if self.stack.len() == self.frame_top {
            return Err(StackError::PopOutOfBounds {
                frame: self.frame_top,
            });
        }

        self.stack.pop().ok_or_else(|| StackError::StackEmpty)
    }

    /// Push a value onto the stack and return its stack index.
    pub fn managed_push(&mut self, value: ValuePtr) -> Result<(), StackError> {
        self.stack.push(value);

        if let Some((managed, slot)) = value.try_into_managed() {
            self.inc_ref(managed, slot)?;
        }

        Ok(())
    }

    /// Pop a value from the stack, freeing it if it's no longer use.
    pub fn managed_pop(&mut self) -> Result<ValuePtr, StackError> {
        if self.stack.len() == self.frame_top {
            return Err(StackError::PopOutOfBounds {
                frame: self.frame_top,
            });
        }

        let value = self.stack.pop().ok_or_else(|| StackError::StackEmpty)?;

        if let Some((managed, slot)) = value.try_into_managed() {
            self.dec_ref(managed, slot)?;
        }

        Ok(value)
    }

    /// Pop a number of values from the stack, freeing them if they are no
    /// longer in use.
    fn managed_popn(&mut self, n: usize) -> Result<(), StackError> {
        if self.stack.len().saturating_sub(self.frame_top) < n {
            return Err(StackError::PopOutOfBounds {
                frame: self.frame_top,
            });
        }

        for _ in 0..n {
            let value = self.stack.pop().ok_or_else(|| StackError::StackEmpty)?;

            if let Some((managed, slot)) = value.try_into_managed() {
                self.dec_ref(managed, slot)?;
            }
        }

        Ok(())
    }

    /// Peek the top of the stack.
    fn peek(&mut self) -> Result<ValuePtr, StackError> {
        self.stack
            .last()
            .copied()
            .ok_or_else(|| StackError::StackEmpty)
    }

    /// Collect any garbage accumulated.
    ///
    /// This will invalidate external value references.
    fn reap(&mut self) -> Result<(), StackError> {
        let mut reap_work = std::mem::take(&mut self.reap_work);

        while !self.reap_queue.is_empty() {
            reap_work.append(&mut self.reap_queue);

            for (managed, slot) in reap_work.drain(..) {
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

                        if let Some(value) = array.value {
                            let value = UnsafeCell::into_inner(value);

                            for value in value {
                                if let Some((managed, slot)) = value.try_into_managed() {
                                    self.dec_ref(managed, slot)?;
                                }
                            }

                            debug_assert!(array.count == 0);
                        }
                    }
                    Managed::Object => {
                        if !self.objects.contains(slot) {
                            log::trace!("trying to free non-existant object: {}", slot);
                            continue;
                        }

                        let object = self.objects.remove(slot);

                        if let Some(value) = object.value {
                            let value = UnsafeCell::into_inner(value);

                            for (_, value) in value {
                                if let Some((managed, slot)) = value.try_into_managed() {
                                    self.dec_ref(managed, slot)?;
                                }
                            }

                            debug_assert!(object.count == 0);
                        }
                    }
                }
            }
        }

        // NB: Hand back the work buffer since it's most likely sized
        // appropriately.
        self.reap_work = reap_work;
        Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    fn stack_copy(&mut self, offset: usize) -> Result<(), VmError> {
        let value = self
            .frame_top
            .checked_add(offset)
            .and_then(|n| self.stack.get(n).copied())
            .ok_or_else(|| VmError::StackOutOfBounds)?;

        if let Some((managed, slot)) = value.try_into_managed() {
            self.inc_ref(managed, slot)?;
        }

        self.stack.push(value);
        Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    fn stack_replace(&mut self, offset: usize) -> Result<(), VmError> {
        let mut value = self.stack.pop().ok_or_else(|| VmError::StackOutOfBounds)?;

        let stack_value = self
            .frame_top
            .checked_add(offset)
            .and_then(|n| self.stack.get_mut(n))
            .ok_or_else(|| VmError::StackOutOfBounds)?;

        mem::swap(stack_value, &mut value);

        // reap old value if necessary.
        if let Some((managed, slot)) = value.try_into_managed() {
            self.dec_ref(managed, slot)?;
        }

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

    /// Allocate and insert an external and return its reference.
    ///
    /// This will leak memory unless the reference is pushed onto the stack to
    /// be managed.
    pub fn external_allocate<T: External>(&mut self, value: T) -> ValuePtr {
        let slot = self.externals.insert(ExternalHolder {
            type_name: type_name::<T>(),
            type_id: TypeId::of::<T>(),
            count: 1,
            access: Access::default(),
            value: Some(Box::new(UnsafeCell::new(value))),
        });

        ValuePtr::Managed(Slot::external(slot))
    }

    impl_ref_count! {
        {String, strings, StringSlotMissing},
        {Array, arrays, ArraySlotMissing},
        {Object, objects, ObjectSlotMissing},
        {External, externals, ExternalSlotMissing},
    }

    impl_slot_functions! {
        strings,
        String,
        StringSlotMissing,
        String,
        string,
        string_allocate,
        string_ref,
        string_clone,
        string_mut,
        string_take,
    }

    impl_slot_functions! {
        arrays,
        Vec<ValuePtr>,
        ArraySlotMissing,
        Array,
        array,
        array_allocate,
        array_ref,
        array_clone,
        array_mut,
        array_take,
    }

    impl_slot_functions! {
        objects,
        HashMap<String, ValuePtr>,
        ObjectSlotMissing,
        Object,
        object,
        object_allocate,
        object_ref,
        object_clone,
        object_mut,
        object_take,
    }

    /// Get a reference of the external value of the given type and the given
    /// slot.
    pub fn external_ref<T: External>(&self, slot: usize) -> Result<Ref<'_, T>, StackError> {
        let holder = self
            .externals
            .get(slot)
            .ok_or_else(|| StackError::ExternalSlotMissing { slot })?;

        holder.access.shared(Managed::External, slot)?;

        let value = match &holder.value {
            Some(value) => value,
            None => {
                holder.access.release_shared();
                return Err(StackError::ExternalSlotMissing { slot });
            }
        };

        // Safety: Caller needs to ensure that they safely call disarm.
        unsafe {
            let external = match (&*value.get()).as_any().downcast_ref::<T>() {
                Some(external) => external,
                None => {
                    // NB: Immediately unshare because the cast failed and we
                    // won't be maintaining access to the type.
                    holder.access.release_shared();

                    return Err(StackError::ExpectedExternalType {
                        expected: type_name::<T>(),
                        actual: holder.type_name,
                    });
                }
            };

            let value = &*(external as *const T);

            Ok(Ref {
                value,
                access: &holder.access,
                guard: (Managed::External, slot),
                guards: &self.guards,
            })
        }
    }

    /// Get a mutable reference of the external value of the given type and the
    /// given slot.
    ///
    /// Mark the given value as mutably used, preventing it from being used
    /// again.
    pub fn external_mut<'out, T: External>(&self, slot: usize) -> Result<Mut<'_, T>, StackError> {
        let holder = self
            .externals
            .get(slot)
            .ok_or_else(|| StackError::ExternalSlotMissing { slot })?;

        holder.access.exclusive(Managed::External, slot)?;

        let value = match &holder.value {
            Some(value) => value,
            None => {
                holder.access.release_exclusive();
                return Err(StackError::ExternalSlotMissing { slot });
            }
        };

        // Safety: Caller needs to ensure that they safely call disarm.
        unsafe {
            let value = (*value.get())
                .as_any_mut()
                .downcast_mut::<T>()
                .ok_or_else(|| StackError::ExpectedExternalType {
                    expected: type_name::<T>(),
                    actual: holder.type_name,
                })?;

            Ok(Mut {
                value,
                access: &holder.access,
                guard: (Managed::External, slot),
                guards: &self.guards,
            })
        }
    }

    /// Get a clone of the given external.
    pub fn external_clone<T: Clone + External>(&self, slot: usize) -> Result<T, StackError> {
        // This is safe since we can rely on the typical reference guarantees of
        // VM.
        if let Some(holder) = self.externals.get(slot) {
            holder.access.test_shared(Managed::Array, slot)?;

            if let Some(value) = &holder.value {
                // Safety: Caller needs to ensure that they safely call disarm.
                unsafe {
                    let external = match (*value.get()).as_any().downcast_ref::<T>() {
                        Some(external) => external,
                        None => {
                            return Err(StackError::ExpectedExternalType {
                                expected: type_name::<T>(),
                                actual: holder.type_name,
                            });
                        }
                    };

                    return Ok(external.clone());
                }
            }
        }

        Err(StackError::ExternalSlotMissing { slot })
    }

    /// Take an external value by dyn, assuming you have exlusive access to it.
    pub fn external_take<T>(&mut self, slot: usize) -> Result<T, StackError>
    where
        T: External,
    {
        let holder = self
            .externals
            .get_mut(slot)
            .ok_or_else(|| StackError::ExternalSlotMissing { slot })?;

        holder.access.exclusive(Managed::External, slot)?;

        let value = match holder.value.take() {
            Some(value) => value,
            None => {
                holder.access.release_exclusive();
                return Err(StackError::ExternalSlotMissing { slot });
            }
        };

        holder.access.release_exclusive();

        // Safety: Caller needs to ensure that they safely call disarm.
        unsafe {
            let value = Box::into_raw(value);

            if let Some(ptr) = (&mut *(*value).get()).as_mut_ptr(TypeId::of::<T>()) {
                return Ok(*Box::from_raw(ptr as *mut T));
            }

            let actual = holder.type_name;

            holder.value = Some(Box::from_raw(value));
            holder.access.clear();

            Err(StackError::ExpectedExternalType {
                expected: type_name::<T>(),
                actual,
            })
        }
    }

    /// Get a reference of the external value of the given type and the given
    /// slot.
    pub fn external_ref_dyn(&self, slot: usize) -> Result<Ref<'_, dyn External>, StackError> {
        let holder = self
            .externals
            .get(slot)
            .ok_or_else(|| StackError::ExternalSlotMissing { slot })?;

        holder.access.shared(Managed::External, slot)?;

        let value = match &holder.value {
            Some(value) => value,
            None => {
                holder.access.release_shared();
                return Err(StackError::ExternalSlotMissing { slot });
            }
        };

        // Safety: Caller needs to ensure that they safely call disarm.
        Ok(Ref {
            value: unsafe { &*value.get() },
            access: &holder.access,
            guard: (Managed::External, slot),
            guards: &self.guards,
        })
    }

    /// Take an external value by dyn, assuming you have exlusive access to it.
    pub fn external_take_dyn(&mut self, slot: usize) -> Result<Box<dyn External>, StackError> {
        let holder = self
            .externals
            .get_mut(slot)
            .ok_or_else(|| StackError::ExternalSlotMissing { slot })?;

        holder.access.exclusive(Managed::External, slot)?;

        let value = match holder.value.take() {
            Some(value) => value,
            None => {
                holder.access.release_exclusive();
                return Err(StackError::ExternalSlotMissing { slot });
            }
        };

        // Safety: Caller needs to ensure that they safely call disarm.
        unsafe {
            let value = Box::into_raw(value);
            return Ok(Box::from_raw((*value).get()));
        }
    }

    /// Access information about an external type, if available.
    pub fn external_type(&self, slot: usize) -> Result<(&'static str, TypeId), StackError> {
        if let Some(holder) = self.externals.get(slot) {
            return Ok((holder.type_name, holder.type_id));
        }

        Err(StackError::ExternalSlotMissing { slot })
    }

    /// Convert a value reference into an owned value.
    pub fn value_take(&mut self, value: ValuePtr) -> Result<Value, StackError> {
        return Ok(match value {
            ValuePtr::Unit => Value::Unit,
            ValuePtr::Integer(integer) => Value::Integer(integer),
            ValuePtr::Float(float) => Value::Float(float),
            ValuePtr::Bool(boolean) => Value::Bool(boolean),
            ValuePtr::Char(c) => Value::Char(c),
            ValuePtr::Managed(managed) => match managed.into_managed() {
                (Managed::String, slot) => Value::String(self.string_take(slot)?),
                (Managed::Array, slot) => {
                    let array = self.array_take(slot)?;
                    Value::Array(value_take_array(self, array)?)
                }
                (Managed::Object, slot) => {
                    let object = self.object_take(slot)?;
                    Value::Object(value_take_object(self, object)?)
                }
                (Managed::External, slot) => Value::External(self.external_take_dyn(slot)?),
            },
            ValuePtr::Type(ty) => Value::Type(ty),
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
            ValuePtr::Managed(managed) => match managed.into_managed() {
                (Managed::String, slot) => ValueRef::String(self.string_ref(slot)?),
                (Managed::Array, slot) => {
                    let array = self.array_ref(slot)?;
                    ValueRef::Array(self.value_array_ref(&*array)?)
                }
                (Managed::Object, slot) => {
                    let object = self.object_ref(slot)?;
                    ValueRef::Object(self.value_object_ref(&*object)?)
                }
                (Managed::External, slot) => ValueRef::External(self.external_ref_dyn(slot)?),
            },
            ValuePtr::Type(ty) => ValueRef::Type(ty),
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
            (ValuePtr::Managed(a), ValuePtr::Managed(b)) => {
                match (a.into_managed(), b.into_managed()) {
                    ((Managed::Array, a), (Managed::Array, b)) => {
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
                    ((Managed::Object, a), (Managed::Object, b)) => {
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
                    ((Managed::String, a), (Managed::String, b)) => {
                        let a = self.string_ref(a)?;
                        let b = self.string_ref(b)?;
                        *a == *b
                    }
                    ((Managed::External, a), (Managed::External, b)) => a == b,
                    _ => false,
                }
            }
            _ => false,
        })
    }

    /// Optimized equality implementation.
    fn eq(&mut self) -> Result<(), VmError> {
        let b = self.managed_pop()?;
        let a = self.managed_pop()?;
        self.unmanaged_push(ValuePtr::Bool(self.value_ptr_eq(a, b)?));
        Ok(())
    }

    /// Optimized inequality implementation.
    fn neq(&mut self) -> Result<(), VmError> {
        let b = self.managed_pop()?;
        let a = self.managed_pop()?;
        self.unmanaged_push(ValuePtr::Bool(!self.value_ptr_eq(a, b)?));
        Ok(())
    }

    fn not(&mut self) -> Result<(), VmError> {
        let value = self.unmanaged_pop()?;

        let value = match value {
            ValuePtr::Bool(value) => ValuePtr::Bool(!value),
            other => {
                let operand = other.type_info(self)?;
                return Err(VmError::UnsupportedUnaryOperation { op: "!", operand });
            }
        };

        self.unmanaged_push(value);
        Ok(())
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
                        let string = {
                            let a = self.string_ref(a)?;
                            let b = self.string_ref(b)?;
                            let mut string = String::with_capacity(a.len() + b.len());
                            string.push_str(a.as_str());
                            string.push_str(b.as_str());
                            string
                        };

                        let value = self.string_allocate(string);
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
            lhs: a.type_info(self)?,
            rhs: b.type_info(self)?,
        })
    }

    /// Perform an index get operation.
    fn index_get(&mut self, target: ValuePtr, index: ValuePtr) -> Result<(), VmError> {
        match (target, index) {
            (ValuePtr::Managed(target), ValuePtr::Managed(index)) => {
                match (target.into_managed(), index.into_managed()) {
                    ((Managed::Object, target), (Managed::String, index)) => {
                        let value = {
                            let object = self.object_ref(target)?;
                            let index = self.string_ref(index)?;
                            object.get(&*index).copied().unwrap_or_default()
                        };

                        self.managed_push(value)?;
                        return Ok(());
                    }
                    _ => (),
                }
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

    /// Perform an index set operation.
    fn index_set(
        &mut self,
        target: ValuePtr,
        index: ValuePtr,
        value: ValuePtr,
    ) -> Result<(), VmError> {
        match (target, index) {
            (ValuePtr::Managed(target), ValuePtr::Managed(index)) => {
                match (target.into_managed(), index.into_managed()) {
                    ((Managed::Object, target), (Managed::String, index)) => {
                        let index = self.string_take(index)?;
                        let mut object = self.object_mut(target)?;
                        object.insert(index, value);
                        return Ok(());
                    }
                    _ => (),
                }
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

    /// Evaluate a single instruction.
    pub async fn eval(
        &mut self,
        inst: &Inst,
        functions: &Functions,
        unit: &Unit,
    ) -> Result<(), VmError> {
        let mut update_ip = true;

        match inst {
            Inst::Not => {
                self.not()?;
            }
            Inst::Add => {
                self.add()?;
            }
            Inst::Sub => {
                let b = self.managed_pop()?;
                let a = self.managed_pop()?;
                self.unmanaged_push(numeric_ops!(self, a - b));
            }
            Inst::Div => {
                let b = self.managed_pop()?;
                let a = self.managed_pop()?;
                self.unmanaged_push(numeric_ops!(self, a / b));
            }
            Inst::Mul => {
                let b = self.managed_pop()?;
                let a = self.managed_pop()?;
                self.unmanaged_push(numeric_ops!(self, a * b));
            }
            Inst::Call { hash, args } => {
                match unit.lookup(*hash) {
                    Some(loc) => {
                        self.push_frame(loc, *args)?;
                        update_ip = false;
                    }
                    None => {
                        let handler = functions
                            .lookup(*hash)
                            .ok_or_else(|| VmError::MissingFunction { hash: *hash })?;

                        let result = handler(self, *args).await;

                        // Safety: We have exclusive access to the VM and
                        // everything that was borrowed during the call can now
                        // be cleared since it's only used in the handler.
                        unsafe {
                            self.disarm();
                        }

                        result?;
                    }
                }
            }
            Inst::CallInstance { hash, args } => {
                let instance = self.peek()?;
                let ty = instance.value_type(self)?;

                let hash = Hash::instance_function(ty, *hash);

                match unit.lookup(hash) {
                    Some(loc) => {
                        self.push_frame(loc, *args)?;
                        update_ip = false;
                    }
                    None => {
                        let handler = functions
                            .lookup(hash)
                            .ok_or_else(|| VmError::MissingFunction { hash: hash })?;

                        let result = handler(self, *args).await;

                        // Safety: We have exclusive access to the VM and
                        // everything that was borrowed during the call can
                        // now be cleared since it's only used in the
                        // handler.
                        unsafe {
                            self.disarm();
                        }

                        result?;
                    }
                }
            }
            Inst::IndexGet => {
                let target = self.managed_pop()?;
                let index = self.managed_pop()?;
                self.index_get(target, index)?;
            }
            Inst::IndexSet => {
                let target = self.managed_pop()?;
                let index = self.managed_pop()?;
                let value = self.managed_pop()?;
                self.index_set(target, index, value)?;
            }
            Inst::Return => {
                // NB: unmanaged because we're effectively moving the value.
                let return_value = self.unmanaged_pop()?;
                self.exited = self.pop_frame()?;
                self.unmanaged_push(return_value);
            }
            Inst::ReturnUnit => {
                self.exited = self.pop_frame()?;
                self.managed_push(ValuePtr::Unit)?;
            }
            Inst::Pop => {
                self.managed_pop()?;
            }
            Inst::PopN { count } => {
                self.managed_popn(*count)?;
            }
            Inst::Clean { count } => {
                let value = self.unmanaged_pop()?;
                self.managed_popn(*count)?;
                self.unmanaged_push(value);
            }
            Inst::Integer { number } => {
                self.managed_push(ValuePtr::Integer(*number))?;
            }
            Inst::Float { number } => {
                self.managed_push(ValuePtr::Float(*number))?;
            }
            Inst::Copy { offset } => {
                self.stack_copy(*offset)?;
            }
            Inst::Replace { offset } => {
                self.stack_replace(*offset)?;
            }
            Inst::Gt => {
                let b = self.managed_pop()?;
                let a = self.managed_pop()?;
                self.unmanaged_push(ValuePtr::Bool(primitive_ops!(self, a > b)));
            }
            Inst::Gte => {
                let b = self.managed_pop()?;
                let a = self.managed_pop()?;
                self.unmanaged_push(ValuePtr::Bool(primitive_ops!(self, a >= b)));
            }
            Inst::Lt => {
                let b = self.managed_pop()?;
                let a = self.managed_pop()?;
                self.unmanaged_push(ValuePtr::Bool(primitive_ops!(self, a < b)));
            }
            Inst::Lte => {
                let b = self.managed_pop()?;
                let a = self.managed_pop()?;
                self.unmanaged_push(ValuePtr::Bool(primitive_ops!(self, a <= b)));
            }
            Inst::Eq => {
                self.eq()?;
            }
            Inst::Neq => {
                self.neq()?;
            }
            Inst::Jump { offset } => {
                self.modify_ip(*offset)?;
                update_ip = false;
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
                self.managed_push(ValuePtr::Unit)?;
            }
            Inst::Bool { value } => {
                self.managed_push(ValuePtr::Bool(*value))?;
            }
            Inst::Array { count } => {
                let mut array = Vec::with_capacity(*count);

                for _ in 0..*count {
                    array.push(self.stack.pop().ok_or_else(|| StackError::StackEmpty)?);
                }

                let value = self.array_allocate(array);
                self.managed_push(value)?;
            }
            Inst::Object { count } => {
                let mut object = HashMap::with_capacity(*count);

                for _ in 0..*count {
                    let key = self.pop_decode()?;
                    let value = self.stack.pop().ok_or_else(|| StackError::StackEmpty)?;
                    object.insert(key, value);
                }

                let value = self.object_allocate(object);
                self.managed_push(value)?;
            }
            Inst::Type { hash } => {
                self.unmanaged_push(ValuePtr::Type(*hash));
            }
            Inst::Char { c } => {
                self.unmanaged_push(ValuePtr::Char(*c));
            }
            Inst::String { slot } => {
                let string = unit
                    .lookup_string(*slot)
                    .ok_or_else(|| VmError::MissingStaticString { slot: *slot })?;
                // TODO: do something sneaky to only allocate the static string once.
                let value = self.string_allocate(string.to_owned());
                self.managed_push(value)?;
            }
            Inst::Is => {
                let b = self.managed_pop()?;
                let a = self.managed_pop()?;

                match (a, b) {
                    (a, ValuePtr::Type(hash)) => {
                        let a = a.value_type(self)?;

                        let type_info = functions
                            .lookup_type(hash)
                            .ok_or_else(|| VmError::MissingType { hash })?;

                        self.unmanaged_push(ValuePtr::Bool(a == type_info.value_type));
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
            }
        }

        if update_ip {
            self.ip += 1;
        }

        self.reap()?;
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
            .field("reap_queue", &self.reap_queue)
            .field("externals", &DebugSlab(&self.externals))
            .field("strings", &DebugSlab(&self.strings))
            .field("arrays", &DebugSlab(&self.arrays))
            .field("objects", &DebugSlab(&self.objects))
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
    pub functions: &'a Functions,
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
            let inst = self
                .unit
                .instruction_at(self.vm.ip())
                .ok_or_else(|| VmError::IpOutOfBounds)?;

            self.vm.eval(inst, self.functions, self.unit).await?;
        }

        let value = self.vm.pop_decode()?;
        self.vm.reap()?;
        Ok(value)
    }

    /// Step the given task until the return value is available.
    pub async fn step(&mut self) -> Result<Option<T>, VmError> {
        let inst = self
            .unit
            .instruction_at(self.vm.ip())
            .ok_or_else(|| VmError::IpOutOfBounds)?;

        self.vm.eval(inst, self.functions, self.unit).await?;

        if self.vm.exited {
            let value = self.vm.pop_decode()?;
            self.vm.reap()?;
            return Ok(Some(value));
        }

        Ok(None)
    }
}
