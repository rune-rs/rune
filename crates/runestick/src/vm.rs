use crate::budget;
use crate::future::SelectFuture;
use crate::unit::UnitFn;
use crate::{
    Args, Awaited, BorrowMut, Bytes, Call, Format, FormatSpec, FromValue, Function, Future,
    Generator, GuardedArgs, Hash, Inst, InstAddress, InstAssignOp, InstFnNameHash, InstOp,
    InstRangeLimits, InstTarget, InstValue, InstVariant, IntoTypeHash, Object, Panic, Protocol,
    Range, RangeLimits, RuntimeContext, Select, Shared, Stack, Stream, Struct, Tuple, TypeCheck,
    Unit, UnitStruct, Value, Variant, VariantData, Vec, VmError, VmErrorKind, VmExecution, VmHalt,
    VmIntegerRepr, VmSendExecution,
};
use std::fmt;
use std::mem;
use std::sync::Arc;
use std::vec;

enum TargetFallback<'a> {
    Value(Value, Value),
    Field(&'a Value, Hash, Value),
    Index(&'a Value, usize, Value),
}

enum TargetValue<'a, 'b> {
    /// Resolved internal target to mutable value.
    Value(&'a mut Value, Value),
    /// Fallback to a different kind of operation.
    Fallback(TargetFallback<'b>),
}

macro_rules! target_value {
    ($vm:ident, $target:expr, $guard:ident, $lhs:ident) => {{
        let rhs = $vm.stack.pop()?;

        match $target {
            InstTarget::Offset(offset) => TargetValue::Value($vm.stack.at_offset_mut(offset)?, rhs),
            InstTarget::TupleField(index) => {
                $lhs = $vm.stack.pop()?;

                if let Some(value) = Vm::try_tuple_like_index_get_mut(&$lhs, index)? {
                    $guard = value;
                    TargetValue::Value(&mut *$guard, rhs)
                } else {
                    TargetValue::Fallback(TargetFallback::Index(&$lhs, index, rhs))
                }
            }
            InstTarget::Field(field) => {
                let field = $vm.unit.lookup_string(field)?;
                $lhs = $vm.stack.pop()?;

                if let Some(value) = Vm::try_object_like_index_get_mut(&$lhs, field)? {
                    $guard = value;
                    TargetValue::Value(&mut *$guard, rhs)
                } else {
                    TargetValue::Fallback(TargetFallback::Field(&$lhs, field.hash(), rhs))
                }
            }
        }
    }};
}

/// A stack which references variables indirectly from a slab.
#[derive(Debug, Clone)]
pub struct Vm {
    /// Context associated with virtual machine.
    pub(crate) context: Arc<RuntimeContext>,
    /// Unit associated with virtual machine.
    pub(crate) unit: Arc<Unit>,
    /// The current instruction pointer.
    ip: usize,
    /// The current stack.
    pub(crate) stack: Stack,
    /// Frames relative to the stack.
    call_frames: vec::Vec<CallFrame>,
}

impl Vm {
    /// Construct a new runestick virtual machine.
    pub const fn new(context: Arc<RuntimeContext>, unit: Arc<Unit>) -> Self {
        Self::new_with_stack(context, unit, Stack::new())
    }

    /// Construct a new runestick virtual machine.
    pub const fn new_with_stack(
        context: Arc<RuntimeContext>,
        unit: Arc<Unit>,
        stack: Stack,
    ) -> Self {
        Self {
            context,
            unit,
            ip: 0,
            stack,
            call_frames: vec::Vec::new(),
        }
    }

    /// Run the given vm to completion.
    ///
    /// If any async instructions are encountered, this will error.
    pub fn complete(self) -> Result<Value, VmError> {
        let mut execution = VmExecution::new(self);
        execution.complete()
    }

    /// Run the given vm to completion with support for async functions.
    pub async fn async_complete(self) -> Result<Value, VmError> {
        let mut execution = VmExecution::new(self);
        execution.async_complete().await
    }

    /// Test if the virtual machine is the same context and unit as specified.
    pub fn is_same(&self, context: &Arc<RuntimeContext>, unit: &Arc<Unit>) -> bool {
        Arc::ptr_eq(&self.context, context) && Arc::ptr_eq(&self.unit, unit)
    }

    /// Set  the current instruction pointer.
    #[inline]
    pub fn set_ip(&mut self, ip: usize) {
        self.ip = ip;
    }

    /// Get the stack.
    #[inline]
    pub fn call_frames(&self) -> &[CallFrame] {
        &self.call_frames
    }

    /// Get the stack.
    #[inline]
    pub fn stack(&self) -> &Stack {
        &self.stack
    }

    /// Get the stack mutably.
    #[inline]
    pub fn stack_mut(&mut self) -> &mut Stack {
        &mut self.stack
    }

    /// Access the context related to the virtual machine.
    #[inline]
    pub fn context(&self) -> &Arc<RuntimeContext> {
        &self.context
    }

    /// Access the underlying unit of the virtual machine.
    #[inline]
    pub fn unit(&self) -> &Arc<Unit> {
        &self.unit
    }

    /// Access the current instruction pointer.
    #[inline]
    pub fn ip(&self) -> usize {
        self.ip
    }

    /// Advance the instruction pointer.
    #[inline]
    pub(crate) fn advance(&mut self) {
        self.ip = self.ip.overflowing_add(1).0;
    }

    /// Reset this virtual machine, freeing all memory used.
    pub fn clear(&mut self) {
        self.ip = 0;
        self.stack.clear();
        self.call_frames.clear();
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

    /// Call the function identified by the given name.
    ///
    /// Computing the function hash from the name can be a bit costly, so it's
    /// worth noting that it can be precalculated:
    ///
    /// ```rust
    /// use runestick::{Hash, Item};
    ///
    /// let name = Hash::type_hash(&["main"]);
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use runestick::{Context, Unit, FromValue, Source};
    /// use std::sync::Arc;
    ///
    /// fn main() -> runestick::Result<()> {
    ///     let context = Context::with_default_modules()?;
    ///     let context = Arc::new(context.runtime());
    ///     let unit = Arc::new(Unit::default());
    ///     // Normally the unit would be created by compiling some source,
    ///     // and since this one is empty it won't do anything.
    ///
    ///     let vm = runestick::Vm::new(context, unit);
    ///
    ///     let output = vm.execute(&["main"], (33i64,))?.complete()?;
    ///     let output = i64::from_value(output)?;
    ///
    ///     println!("output: {}", output);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// You can use a `Vec<Value>` to provide a variadic collection of
    /// arguments.
    ///
    /// ```rust,no_run
    /// use runestick::{Context, Unit, FromValue, Source, ToValue};
    /// use std::sync::Arc;
    ///
    /// fn main() -> runestick::Result<()> {
    ///     let context = Context::with_default_modules()?;
    ///     let context = Arc::new(context.runtime());
    ///     let unit = Arc::new(Unit::default());
    ///     // Normally the unit would be created by compiling some source,
    ///     // and since this one is empty it won't do anything.
    ///
    ///     let vm = runestick::Vm::new(context, unit);
    ///
    ///     let mut args = Vec::new();
    ///     args.push(1u32.to_value()?);
    ///     args.push(String::from("Hello World").to_value()?);
    ///
    ///     let output = vm.execute(&["main"], args)?.complete()?;
    ///     let output = i64::from_value(output)?;
    ///
    ///     println!("output: {}", output);
    ///     Ok(())
    /// }
    /// ```
    pub fn execute<A, N>(mut self, name: N, args: A) -> Result<VmExecution, VmError>
    where
        N: IntoTypeHash,
        A: Args,
    {
        self.set_entrypoint(name, args.count())?;
        args.into_stack(&mut self.stack)?;
        Ok(self.into_execution())
    }

    /// An `execute` variant that returns an execution which implements
    /// [`Send`], allowing it to be sent and executed on a different thread.
    ///
    /// This is accomplished by preventing values escaping from being
    /// non-exclusively sent with the execution or escaping the execution. We
    /// only support encoding arguments which themselves are `Send`.
    pub fn send_execute<A, N>(mut self, name: N, args: A) -> Result<VmSendExecution, VmError>
    where
        N: IntoTypeHash,
        A: Send + Args,
    {
        // Safety: make sure the stack is clear, preventing any values from
        // being sent along with the virtual machine.
        self.stack.clear();

        let execution = self.execute(name, args)?;
        Ok(VmSendExecution(execution))
    }

    /// Call the given function immediately, returning the produced value.
    ///
    /// This function permits for using references since it doesn't defer its
    /// execution.
    ///
    /// # Panics
    ///
    /// If any of the arguments passed in are references, and that references is
    /// captured somewhere in the call as [`Mut<T>`] or [`Ref<T>`]
    /// this call will panic as we are trying to free the metadata relatedc to
    /// the reference.
    ///
    /// [`Mut<T>`]: crate::Mut
    /// [`Ref<T>`]: crate::Ref
    pub fn call<A, N>(mut self, name: N, args: A) -> Result<Value, VmError>
    where
        N: IntoTypeHash,
        A: GuardedArgs,
    {
        self.set_entrypoint(name, args.count())?;

        // Safety: We hold onto the guard until the vm has completed.
        let guard = unsafe { args.unsafe_into_stack(&mut self.stack)? };

        let value = self.into_execution().complete()?;

        // Note: this might panic if something in the vm is holding on to a
        // reference of the value. We should prevent it from being possible to
        // take any owned references to values held by this.
        drop(guard);
        Ok(value)
    }

    /// Convert this virtual machine into an execution.
    fn into_execution(self) -> VmExecution {
        VmExecution::new(self)
    }

    /// Call the given function immediately asynchronously, returning the
    /// produced value.
    ///
    /// This function permits for using references since it doesn't defer its
    /// execution.
    ///
    /// # Panics
    ///
    /// If any of the arguments passed in are references, and that references is
    /// captured somewhere in the call as [`Mut<T>`] or [`Ref<T>`]
    /// this call will panic as we are trying to free the metadata relatedc to
    /// the reference.
    ///
    /// [`Mut<T>`]: crate::Mut
    /// [`Ref<T>`]: crate::Ref
    pub async fn async_call<A, N>(mut self, name: N, args: A) -> Result<Value, VmError>
    where
        N: IntoTypeHash,
        A: GuardedArgs,
    {
        self.set_entrypoint(name, args.count())?;

        // Safety: We hold onto the guard until the vm has completed.
        let guard = unsafe { args.unsafe_into_stack(&mut self.stack)? };

        let value = VmExecution::new(self).async_complete().await?;

        // Note: this might panic if something in the vm is holding on to a
        // reference of the value. We should prevent it from being possible to
        // take any owned references to values held by this.
        drop(guard);
        Ok(value)
    }

    // Update the instruction pointer to match the function matching the given
    // name and check that the number of argument matches.
    fn set_entrypoint<N>(&mut self, name: N, count: usize) -> Result<(), VmError>
    where
        N: IntoTypeHash,
    {
        let hash = name.into_type_hash();

        let info = self.unit.lookup(hash).ok_or_else(|| {
            VmError::from(VmErrorKind::MissingEntry {
                hash,
                item: name.into_item(),
            })
        })?;

        let offset = match info {
            // NB: we ignore the calling convention.
            // everything is just async when called externally.
            UnitFn::Offset {
                offset,
                args: expected,
                ..
            } => {
                Self::check_args(count, expected)?;
                offset
            }
            _ => {
                return Err(VmError::from(VmErrorKind::MissingFunction { hash }));
            }
        };

        self.ip = offset;
        self.stack.clear();
        Ok(())
    }

    /// Helper function to call an instance function.
    #[inline(always)]
    pub(crate) fn call_instance_fn<H, A>(
        &mut self,
        target: Value,
        hash: H,
        args: A,
    ) -> Result<bool, VmError>
    where
        H: IntoTypeHash,
        A: GuardedArgs,
    {
        let count = args.count() + 1;
        let type_hash = target.type_hash()?;
        self.stack.push(target.clone());

        // Safety: We hold onto the guard for the duration of this call.
        let _guard = unsafe { args.unsafe_into_stack(&mut self.stack)? };

        let hash = Hash::instance_function(type_hash, hash.into_type_hash());

        if let Some(UnitFn::Offset {
            offset,
            call,
            args: expected,
        }) = self.unit.lookup(hash)
        {
            Self::check_args(count, expected)?;
            self.call_offset_fn(offset, call, count)?;
            return Ok(true);
        }

        let handler = match self.context.lookup(hash) {
            Some(handler) => handler,
            None => {
                // NB: restore the stack
                self.stack.popn(count)?;
                return Ok(false);
            }
        };

        handler(&mut self.stack, count)?;
        Ok(true)
    }

    /// Helper to call a field function.
    #[inline(always)]
    fn call_field_fn<H, A>(
        &mut self,
        protocol: Protocol,
        target: &Value,
        hash: H,
        args: A,
    ) -> Result<bool, VmError>
    where
        H: IntoTypeHash,
        A: Args,
    {
        let count = args.count() + 1;
        self.stack.push(target.clone());
        args.into_stack(&mut self.stack)?;

        let hash = Hash::field_fn(protocol, target.type_hash()?, hash.into_type_hash());

        let handler = match self.context.lookup(hash) {
            Some(handler) => handler,
            None => {
                // NB: restore the stack
                self.stack.popn(count)?;
                return Ok(false);
            }
        };

        handler(&mut self.stack, count)?;
        Ok(true)
    }

    fn internal_boolean_ops(
        &mut self,
        int_op: fn(i64, i64) -> bool,
        float_op: fn(f64, f64) -> bool,
        op: &'static str,
        lhs: InstAddress,
        rhs: InstAddress,
    ) -> Result<(), VmError> {
        let rhs = self.stack.address(rhs)?;
        let lhs = self.stack.address(lhs)?;

        let out = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => int_op(lhs, rhs),
            (Value::Float(lhs), Value::Float(rhs)) => float_op(lhs, rhs),
            (lhs, rhs) => {
                return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                    op,
                    lhs: lhs.type_info()?,
                    rhs: rhs.type_info()?,
                }))
            }
        };

        self.stack.push(out);
        Ok(())
    }

    /// Push a new call frame.
    ///
    /// This will cause the `args` number of elements on the stack to be
    /// associated and accessible to the new call frame.
    pub(crate) fn push_call_frame(&mut self, ip: usize, args: usize) -> Result<(), VmError> {
        let stack_top = self.stack.swap_stack_bottom(args)?;

        self.call_frames.push(CallFrame {
            ip: self.ip,
            stack_bottom: stack_top,
        });

        self.ip = ip.overflowing_sub(1).0;
        Ok(())
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

        self.stack.pop_stack_top(frame.stack_bottom)?;
        self.ip = frame.ip;
        Ok(false)
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_object_like_index_get(target: &Value, field: &str) -> Result<Option<Value>, VmError> {
        let value = match &target {
            Value::Object(target) => target.borrow_ref()?.get(field).cloned(),
            Value::Struct(target) => target.borrow_ref()?.get(field).cloned(),
            Value::Variant(variant) => match variant.borrow_ref()?.data() {
                VariantData::Struct(target) => target.get(field).cloned(),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return Err(VmError::from(VmErrorKind::MissingField {
                    target: target.type_info()?,
                    field: field.to_owned(),
                }));
            }
        };

        Ok(Some(value))
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
            Value::TupleStruct(tuple_struct) => {
                let tuple_struct = tuple_struct.borrow_ref()?;
                tuple_struct.data().get(index).cloned()
            }
            Value::Variant(variant) => {
                let variant = variant.borrow_ref()?;

                match variant.data() {
                    VariantData::Tuple(tuple) => tuple.get(index).cloned(),
                    _ => return Ok(None),
                }
            }
            _ => return Ok(None),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return Err(VmError::from(VmErrorKind::MissingIndex {
                    target: target.type_info()?,
                    index: VmIntegerRepr::from(index),
                }));
            }
        };

        Ok(Some(value))
    }

    /// Implementation of getting a mutable value out of a tuple-like value.
    fn try_tuple_like_index_get_mut(
        target: &Value,
        index: usize,
    ) -> Result<Option<BorrowMut<'_, Value>>, VmError> {
        let value = match target {
            Value::Unit => None,
            Value::Tuple(tuple) => {
                let tuple = tuple.borrow_mut()?;

                BorrowMut::try_map(tuple, |tuple| tuple.get_mut(index))
            }
            Value::Vec(vec) => {
                let vec = vec.borrow_mut()?;

                BorrowMut::try_map(vec, |vec| vec.get_mut(index))
            }
            Value::Result(result) => {
                let result = result.borrow_mut()?;

                BorrowMut::try_map(result, |result| match result {
                    Ok(value) if index == 0 => Some(value),
                    Err(value) if index == 0 => Some(value),
                    _ => None,
                })
            }
            Value::Option(option) => {
                let option = option.borrow_mut()?;

                BorrowMut::try_map(option, |option| match option {
                    Some(value) if index == 0 => Some(value),
                    _ => None,
                })
            }
            Value::GeneratorState(state) => {
                use crate::GeneratorState::*;
                let state = state.borrow_mut()?;

                BorrowMut::try_map(state, |state| match state {
                    Yielded(value) if index == 0 => Some(value),
                    Complete(value) if index == 0 => Some(value),
                    _ => None,
                })
            }
            Value::TupleStruct(tuple_struct) => {
                let tuple_struct = tuple_struct.borrow_mut()?;

                BorrowMut::try_map(tuple_struct, |tuple_struct| tuple_struct.get_mut(index))
            }
            Value::Variant(variant) => {
                let variant = variant.borrow_mut()?;

                BorrowMut::try_map(variant, |variant| match variant.data_mut() {
                    VariantData::Tuple(tuple) => tuple.get_mut(index),
                    _ => None,
                })
            }
            _ => return Ok(None),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return Err(VmError::from(VmErrorKind::MissingIndex {
                    target: target.type_info()?,
                    index: VmIntegerRepr::from(index),
                }));
            }
        };

        Ok(Some(value))
    }

    /// Implementation of getting a mutable string index on an object-like type.
    fn try_object_like_index_get_mut<'a>(
        target: &'a Value,
        field: &str,
    ) -> Result<Option<BorrowMut<'a, Value>>, VmError> {
        let value = match &target {
            Value::Object(target) => {
                let target = target.borrow_mut()?;
                BorrowMut::try_map(target, |target| target.get_mut(field))
            }
            Value::Struct(target) => {
                let target = target.borrow_mut()?;
                BorrowMut::try_map(target, |target| target.get_mut(field))
            }
            Value::Variant(target) => {
                BorrowMut::try_map(target.borrow_mut()?, |target| match target.data_mut() {
                    VariantData::Struct(st) => st.get_mut(field),
                    _ => None,
                })
            }
            _ => return Ok(None),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return Err(VmError::from(VmErrorKind::MissingField {
                    target: target.type_info()?,
                    field: field.to_owned(),
                }));
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
            Value::TupleStruct(tuple_struct) => {
                let mut tuple_struct = tuple_struct.borrow_mut()?;

                if let Some(target) = tuple_struct.get_mut(index) {
                    *target = value;
                    return Ok(true);
                }

                Ok(false)
            }
            Value::Variant(variant) => {
                let mut variant = variant.borrow_mut()?;

                if let VariantData::Tuple(data) = variant.data_mut() {
                    if let Some(target) = data.get_mut(index) {
                        *target = value;
                        return Ok(true);
                    }
                }

                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_object_slot_index_get(
        &mut self,
        target: &Value,
        string_slot: usize,
    ) -> Result<Option<Value>, VmError> {
        let index = self.unit.lookup_string(string_slot)?;

        Ok(match target {
            Value::Object(object) => {
                let object = object.borrow_ref()?;

                match object.get(&***index).cloned() {
                    Some(value) => Some(value),
                    None => {
                        return Err(VmError::from(VmErrorKind::ObjectIndexMissing {
                            slot: string_slot,
                        }));
                    }
                }
            }
            Value::Struct(typed_object) => {
                let typed_object = typed_object.borrow_ref()?;

                match typed_object.get(&***index).cloned() {
                    Some(value) => Some(value),
                    None => {
                        return Err(VmError::from(VmErrorKind::ObjectIndexMissing {
                            slot: string_slot,
                        }));
                    }
                }
            }
            Value::Variant(variant) => {
                let variant = variant.borrow_ref()?;

                match variant.data() {
                    VariantData::Struct(data) => match data.get(&***index).cloned() {
                        Some(value) => Some(value),
                        None => {
                            return Err(VmError::from(VmErrorKind::ObjectIndexMissing {
                                slot: string_slot,
                            }));
                        }
                    },
                    _ => None,
                }
            }
            target => {
                let hash = index.hash();

                if self.call_field_fn(Protocol::GET, target, hash, ())? {
                    Some(self.stack.pop()?)
                } else {
                    None
                }
            }
        })
    }

    fn try_object_slot_index_set(
        &mut self,
        target: &Value,
        string_slot: usize,
        value: Value,
    ) -> Result<Option<()>, VmError> {
        let field = self.unit.lookup_string(string_slot)?;

        Ok(match target {
            Value::Object(object) => {
                let mut object = object.borrow_mut()?;
                object.insert(field.as_str().to_owned(), value);
                return Ok(Some(()));
            }
            Value::Struct(typed_object) => {
                let mut typed_object = typed_object.borrow_mut()?;

                if let Some(v) = typed_object.get_mut(field.as_str()) {
                    *v = value;
                    return Ok(Some(()));
                }

                return Err(VmError::from(VmErrorKind::MissingField {
                    field: field.as_str().to_owned(),
                    target: typed_object.type_info(),
                }));
            }
            Value::Variant(variant) => {
                let mut variant = variant.borrow_mut()?;

                if let VariantData::Struct(data) = variant.data_mut() {
                    if let Some(v) = data.get_mut(field.as_str()) {
                        *v = value;
                        return Ok(Some(()));
                    }
                }

                return Err(VmError::from(VmErrorKind::MissingField {
                    field: field.as_str().to_owned(),
                    target: variant.type_info(),
                }));
            }
            target => {
                let hash = field.hash();

                if self.call_field_fn(Protocol::SET, target, hash, (value,))? {
                    self.stack.pop()?;
                    Some(())
                } else {
                    None
                }
            }
        })
    }

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
            (TypeCheck::Type(hash), value) => match value {
                Value::UnitStruct(empty) => {
                    if empty.borrow_ref()?.rtti.hash != hash {
                        return Ok(None);
                    }

                    Some(f(&[]))
                }
                Value::TupleStruct(tuple_struct) => {
                    let tuple_struct = tuple_struct.borrow_ref()?;

                    if tuple_struct.rtti.hash != hash {
                        return Ok(None);
                    }

                    Some(f(tuple_struct.data()))
                }
                _ => None,
            },
            (TypeCheck::Variant(hash), Value::Variant(variant)) => {
                let variant = variant.borrow_ref()?;

                if variant.rtti().hash != hash {
                    return Ok(None);
                }

                match variant.data() {
                    VariantData::Unit => Some(f(&[])),
                    VariantData::Tuple(tuple) => Some(f(&*tuple)),
                    _ => None,
                }
            }
            (TypeCheck::Unit, Value::Unit) => Some(f(&[])),
            _ => None,
        })
    }

    /// Internal implementation of the instance check.
    fn is_instance(&mut self, lhs: InstAddress, rhs: InstAddress) -> Result<bool, VmError> {
        let b = self.stack.address(rhs)?;
        let a = self.stack.address(lhs)?;

        let hash = match b {
            Value::Type(hash) => hash,
            _ => {
                return Err(VmError::from(VmErrorKind::UnsupportedIs {
                    value: a.type_info()?,
                    test_type: b.type_info()?,
                }));
            }
        };

        Ok(a.type_hash()? == hash)
    }

    fn internal_boolean_op(
        &mut self,
        bool_op: impl FnOnce(bool, bool) -> bool,
        op: &'static str,
        lhs: InstAddress,
        rhs: InstAddress,
    ) -> Result<(), VmError> {
        let rhs = self.stack.address(rhs)?;
        let lhs = self.stack.address(lhs)?;

        let out = match (lhs, rhs) {
            (Value::Bool(lhs), Value::Bool(rhs)) => bool_op(lhs, rhs),
            (lhs, rhs) => {
                return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                    op,
                    lhs: lhs.type_info()?,
                    rhs: rhs.type_info()?,
                }));
            }
        };

        self.stack.push(out);
        Ok(())
    }

    fn on_object_keys<F, O>(
        &mut self,
        type_check: TypeCheck,
        slot: usize,
        f: F,
    ) -> Result<Option<O>, VmError>
    where
        F: FnOnce(&Object, &[String]) -> O,
    {
        let value = self.stack.pop()?;

        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or(VmErrorKind::MissingStaticObjectKeys { slot })?;

        match (type_check, value) {
            (TypeCheck::Object, Value::Object(object)) => {
                let object = object.borrow_ref()?;
                return Ok(Some(f(&*object, keys)));
            }
            (TypeCheck::Type(hash), Value::Struct(typed_object)) => {
                let typed_object = typed_object.borrow_ref()?;

                if typed_object.type_hash() == hash {
                    return Ok(Some(f(typed_object.data(), keys)));
                }
            }
            (TypeCheck::Variant(hash), Value::Variant(variant)) => {
                let variant = variant.borrow_ref()?;

                if variant.rtti().hash == hash {
                    if let VariantData::Struct(st) = variant.data() {
                        return Ok(Some(f(st, keys)));
                    }
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
        self.stack.push(Generator::new(vm));
        Ok(())
    }

    /// Construct a stream from calling a function.
    fn call_stream_fn(&mut self, offset: usize, args: usize) -> Result<(), VmError> {
        let stack = self.stack.drain_stack_top(args)?.collect::<Stack>();
        let mut vm = Self::new_with_stack(self.context.clone(), self.unit.clone(), stack);
        vm.ip = offset;
        self.stack.push(Stream::new(vm));
        Ok(())
    }

    /// Construct a future from calling a function.
    fn call_async_fn(&mut self, offset: usize, args: usize) -> Result<(), VmError> {
        let stack = self.stack.drain_stack_top(args)?.collect::<Stack>();
        let mut vm = Self::new_with_stack(self.context.clone(), self.unit.clone(), stack);
        vm.ip = offset;
        self.stack.push(Future::new(vm.async_complete()));
        Ok(())
    }

    /// Helper function to call the function at the given offset.
    pub(crate) fn call_offset_fn(
        &mut self,
        offset: usize,
        call: Call,
        args: usize,
    ) -> Result<(), VmError> {
        match call {
            Call::Async => {
                self.call_async_fn(offset, args)?;
            }
            Call::Stream => {
                self.call_stream_fn(offset, args)?;
            }
            Call::Generator => {
                self.call_generator_fn(offset, args)?;
            }
            Call::Immediate => {
                self.push_call_frame(offset, args)?;
            }
        }

        Ok(())
    }

    fn internal_num_assign(
        &mut self,
        target: InstTarget,
        protocol: Protocol,
        error: fn() -> VmErrorKind,
        integer_op: fn(i64, i64) -> Option<i64>,
        float_op: fn(f64, f64) -> f64,
    ) -> Result<(), VmError> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs) {
            TargetValue::Value(lhs, rhs) => match (lhs, rhs) {
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
                (lhs, rhs) => TargetFallback::Value(lhs.clone(), rhs),
            },
            TargetValue::Fallback(fallback) => fallback,
        };

        self.target_fallback(fallback, protocol)
    }

    /// Execute a fallback operation.
    fn target_fallback(
        &mut self,
        fallback: TargetFallback<'_>,
        protocol: Protocol,
    ) -> Result<(), VmError> {
        match fallback {
            TargetFallback::Value(lhs, rhs) => {
                if !self.call_instance_fn(lhs.clone(), protocol, (&rhs,))? {
                    return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                        op: protocol.name,
                        lhs: lhs.type_info()?,
                        rhs: rhs.type_info()?,
                    }));
                }

                let value = self.stack.pop()?;
                <()>::from_value(value)?;
                Ok(())
            }
            TargetFallback::Field(lhs, hash, rhs) => {
                if !self.call_field_fn(protocol, lhs, hash, (rhs,))? {
                    return Err(VmError::from(VmErrorKind::UnsupportedObjectSlotIndexGet {
                        target: lhs.type_info()?,
                    }));
                }

                let value = self.stack.pop()?;
                <()>::from_value(value)?;
                Ok(())
            }
            TargetFallback::Index(lhs, ..) => {
                Err(VmError::from(VmErrorKind::UnsupportedTupleIndexGet {
                    target: lhs.type_info()?,
                }))
            }
        }
    }

    /// Internal impl of a numeric operation.
    fn internal_num(
        &mut self,
        protocol: Protocol,
        error: fn() -> VmErrorKind,
        integer_op: fn(i64, i64) -> Option<i64>,
        float_op: fn(f64, f64) -> f64,
        lhs: InstAddress,
        rhs: InstAddress,
    ) -> Result<(), VmError> {
        let rhs = self.stack.address(rhs)?;
        let lhs = self.stack.address(lhs)?;

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                self.stack.push(integer_op(lhs, rhs).ok_or_else(error)?);
                return Ok(());
            }
            (Value::Float(lhs), Value::Float(rhs)) => {
                self.stack.push(float_op(lhs, rhs));
                return Ok(());
            }
            (lhs, rhs) => (lhs, rhs),
        };

        if !self.call_instance_fn(lhs.clone(), protocol, (&rhs,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                op: protocol.name,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }));
        }

        Ok(())
    }

    /// Internal impl of a numeric operation.
    fn internal_infallible_bitwise(
        &mut self,
        protocol: Protocol,
        integer_op: fn(i64, i64) -> i64,
        lhs: InstAddress,
        rhs: InstAddress,
    ) -> Result<(), VmError> {
        let rhs = self.stack.address(rhs)?;
        let lhs = self.stack.address(lhs)?;

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                self.stack.push(integer_op(lhs, rhs));
                return Ok(());
            }
            (lhs, rhs) => (lhs, rhs),
        };

        if !self.call_instance_fn(lhs.clone(), protocol, (&rhs,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                op: protocol.name,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }));
        }

        Ok(())
    }

    /// Internal impl of a numeric operation.
    fn internal_infallible_bitwise_bool(
        &mut self,
        protocol: Protocol,
        integer_op: fn(i64, i64) -> i64,
        bool_op: fn(bool, bool) -> bool,
        lhs: InstAddress,
        rhs: InstAddress,
    ) -> Result<(), VmError> {
        let rhs = self.stack.address(rhs)?;
        let lhs = self.stack.address(lhs)?;

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                self.stack.push(integer_op(lhs, rhs));
                return Ok(());
            }
            (Value::Bool(lhs), Value::Bool(rhs)) => {
                self.stack.push(bool_op(lhs, rhs));
                return Ok(());
            }
            (lhs, rhs) => (lhs, rhs),
        };

        if !self.call_instance_fn(lhs.clone(), protocol, (&rhs,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                op: protocol.name,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }));
        }

        Ok(())
    }

    fn internal_infallible_bitwise_assign(
        &mut self,
        target: InstTarget,
        protocol: Protocol,
        integer_op: fn(&mut i64, i64),
    ) -> Result<(), VmError> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs) {
            TargetValue::Value(lhs, rhs) => match (lhs, rhs) {
                (Value::Integer(lhs), Value::Integer(rhs)) => {
                    integer_op(lhs, rhs);
                    return Ok(());
                }
                (lhs, rhs) => TargetFallback::Value(lhs.clone(), rhs),
            },
            TargetValue::Fallback(fallback) => fallback,
        };

        self.target_fallback(fallback, protocol)
    }

    fn internal_bitwise(
        &mut self,
        protocol: Protocol,
        error: fn() -> VmErrorKind,
        integer_op: fn(i64, i64) -> Option<i64>,
        lhs: InstAddress,
        rhs: InstAddress,
    ) -> Result<(), VmError> {
        let rhs = self.stack.address(rhs)?;
        let lhs = self.stack.address(lhs)?;

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                self.stack.push(integer_op(lhs, rhs).ok_or_else(error)?);
                return Ok(());
            }
            (lhs, rhs) => (lhs, rhs),
        };

        if !self.call_instance_fn(lhs.clone(), protocol, (&rhs,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                op: protocol.name,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }));
        }

        Ok(())
    }

    fn internal_bitwise_assign(
        &mut self,
        target: InstTarget,
        protocol: Protocol,
        error: fn() -> VmErrorKind,
        integer_op: fn(i64, i64) -> Option<i64>,
    ) -> Result<(), VmError> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs) {
            TargetValue::Value(lhs, rhs) => match (lhs, rhs) {
                (Value::Integer(lhs), Value::Integer(rhs)) => {
                    let out = integer_op(*lhs, rhs).ok_or_else(error)?;
                    *lhs = out;
                    return Ok(());
                }
                (lhs, rhs) => TargetFallback::Value(lhs.clone(), rhs),
            },
            TargetValue::Fallback(fallback) => fallback,
        };

        self.target_fallback(fallback, protocol)
    }

    /// Check that arguments matches expected or raise the appropriate error.
    fn check_args(args: usize, expected: usize) -> Result<(), VmError> {
        if args != expected {
            return Err(VmError::from(VmErrorKind::BadArgumentCount {
                actual: args,
                expected,
            }));
        }

        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_await(&mut self) -> Result<Shared<Future>, VmError> {
        let value = self.stack.pop()?;
        value.into_shared_future()
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_select(&mut self, len: usize) -> Result<Option<Select>, VmError> {
        let futures = futures_util::stream::FuturesUnordered::new();

        for (branch, value) in self.stack.drain_stack_top(len)?.enumerate() {
            let future = value.into_shared_future()?.into_mut()?;

            if !future.is_completed() {
                futures.push(SelectFuture::new(branch, future));
            }
        }

        // NB: nothing to poll.
        if futures.is_empty() {
            self.stack.push(());
            return Ok(None);
        }

        Ok(Some(Select::new(futures)))
    }

    /// Pop a number of values from the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_popn(&mut self, n: usize) -> Result<(), VmError> {
        self.stack.popn(n)?;
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_push(&mut self, value: InstValue) -> Result<(), VmError> {
        self.stack.push(value.into_value());
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_pop(&mut self) -> Result<(), VmError> {
        self.stack.pop()?;
        Ok(())
    }

    /// pop-and-jump-if-not instruction.
    #[cfg_attr(feature = "bench", inline(never))]
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
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_clean(&mut self, n: usize) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        self.op_popn(n)?;
        self.stack.push(value);
        Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_copy(&mut self, offset: usize) -> Result<(), VmError> {
        let value = self.stack.at_offset(offset)?.clone();
        self.stack.push(value);
        Ok(())
    }

    /// Move a value from a position relative to the top of the stack, to the
    /// top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_move(&mut self, offset: usize) -> Result<(), VmError> {
        let value = self.stack.at_offset(offset)?.clone();
        self.stack.push(value.take()?);
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_drop(&mut self, offset: usize) -> Result<(), VmError> {
        let _ = self.stack.at_offset(offset)?;
        Ok(())
    }

    /// Duplicate the value at the top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_dup(&mut self) -> Result<(), VmError> {
        let value = self.stack.last()?.clone();
        self.stack.push(value);
        Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_replace(&mut self, offset: usize) -> Result<(), VmError> {
        let mut value = self.stack.pop()?;
        let stack_value = self.stack.at_offset_mut(offset)?;
        mem::swap(stack_value, &mut value);
        Ok(())
    }

    /// Perform a jump operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_jump(&mut self, offset: isize) -> Result<(), VmError> {
        self.modify_ip(offset)?;
        Ok(())
    }

    /// Perform a conditional jump operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_jump_if(&mut self, offset: isize) -> Result<(), VmError> {
        if self.stack.pop()?.into_bool()? {
            self.modify_ip(offset)?;
        }

        Ok(())
    }

    /// Perform a conditional jump operation. Pops the stack if the jump is
    /// not performed.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_jump_if_or_pop(&mut self, offset: isize) -> Result<(), VmError> {
        if self.stack.last()?.as_bool()? {
            self.modify_ip(offset)?;
        } else {
            self.stack.pop()?;
        }

        Ok(())
    }

    /// Perform a conditional jump operation. Pops the stack if the jump is
    /// not performed.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_jump_if_not_or_pop(&mut self, offset: isize) -> Result<(), VmError> {
        if !self.stack.last()?.as_bool()? {
            self.modify_ip(offset)?;
        } else {
            self.stack.pop()?;
        }

        Ok(())
    }

    /// Perform a branch-conditional jump operation.
    #[cfg_attr(feature = "bench", inline(never))]
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
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_vec(&mut self, count: usize) -> Result<(), VmError> {
        let vec = Vec::from(self.stack.pop_sequence(count)?);
        self.stack.push(Shared::new(vec));
        Ok(())
    }

    /// Construct a new tuple.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple(&mut self, count: usize) -> Result<(), VmError> {
        let tuple = self.stack.pop_sequence(count)?;
        self.stack.push(Tuple::from(tuple));
        Ok(())
    }

    /// Construct a new tuple with a fixed number of arguments.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_n(&mut self, args: &[InstAddress]) -> Result<(), VmError> {
        let mut tuple = vec![Value::Unit; args.len()];

        for (n, arg) in args.iter().enumerate().rev() {
            tuple[n] = self.stack.address(*arg)?;
        }

        self.stack.push(Tuple::from(tuple));
        Ok(())
    }

    /// Push the tuple that is on top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_push_tuple(&mut self) -> Result<(), VmError> {
        let tuple = self.stack.pop()?.into_tuple()?;
        self.stack.extend(tuple.borrow_ref()?.iter().cloned());
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_not(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let value = match value {
            Value::Bool(value) => Value::from(!value),
            Value::Integer(value) => Value::from(!value),
            other => {
                let operand = other.type_info()?;
                return Err(VmError::from(VmErrorKind::UnsupportedUnaryOperation {
                    op: "!",
                    operand,
                }));
            }
        };

        self.stack.push(value);
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_neg(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let value = match value {
            Value::Float(value) => Value::from(-value),
            Value::Integer(value) => Value::from(-value),
            other => {
                let operand = other.type_info()?;
                return Err(VmError::from(VmErrorKind::UnsupportedUnaryOperation {
                    op: "-",
                    operand,
                }));
            }
        };

        self.stack.push(value);
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_op(&mut self, op: InstOp, lhs: InstAddress, rhs: InstAddress) -> Result<(), VmError> {
        use std::convert::TryFrom as _;

        match op {
            InstOp::Add => {
                self.internal_num(
                    Protocol::ADD,
                    || VmErrorKind::Overflow,
                    i64::checked_add,
                    std::ops::Add::add,
                    lhs,
                    rhs,
                )?;
            }
            InstOp::Sub => {
                self.internal_num(
                    Protocol::SUB,
                    || VmErrorKind::Underflow,
                    i64::checked_sub,
                    std::ops::Sub::sub,
                    lhs,
                    rhs,
                )?;
            }
            InstOp::Mul => {
                self.internal_num(
                    Protocol::MUL,
                    || VmErrorKind::Overflow,
                    i64::checked_mul,
                    std::ops::Mul::mul,
                    lhs,
                    rhs,
                )?;
            }
            InstOp::Div => {
                self.internal_num(
                    Protocol::DIV,
                    || VmErrorKind::DivideByZero,
                    i64::checked_div,
                    std::ops::Div::div,
                    lhs,
                    rhs,
                )?;
            }
            InstOp::Rem => {
                self.internal_num(
                    Protocol::REM,
                    || VmErrorKind::DivideByZero,
                    i64::checked_rem,
                    std::ops::Rem::rem,
                    lhs,
                    rhs,
                )?;
            }
            InstOp::BitAnd => {
                use std::ops::BitAnd as _;
                self.internal_infallible_bitwise_bool(
                    Protocol::BIT_AND,
                    i64::bitand,
                    bool::bitand,
                    lhs,
                    rhs,
                )?;
            }
            InstOp::BitXor => {
                use std::ops::BitXor as _;
                self.internal_infallible_bitwise_bool(
                    Protocol::BIT_XOR,
                    i64::bitxor,
                    bool::bitxor,
                    lhs,
                    rhs,
                )?;
            }
            InstOp::BitOr => {
                use std::ops::BitOr as _;
                self.internal_infallible_bitwise_bool(
                    Protocol::BIT_OR,
                    i64::bitor,
                    bool::bitor,
                    lhs,
                    rhs,
                )?;
            }
            InstOp::Shl => {
                self.internal_bitwise(
                    Protocol::SHL,
                    || VmErrorKind::Overflow,
                    |a, b| a.checked_shl(u32::try_from(b).ok()?),
                    lhs,
                    rhs,
                )?;
            }
            InstOp::Shr => {
                self.internal_infallible_bitwise(Protocol::SHR, std::ops::Shr::shr, lhs, rhs)?;
            }
            InstOp::Gt => {
                self.internal_boolean_ops(|a, b| a > b, |a, b| a > b, ">", lhs, rhs)?;
            }
            InstOp::Gte => {
                self.internal_boolean_ops(|a, b| a >= b, |a, b| a >= b, ">=", lhs, rhs)?;
            }
            InstOp::Lt => {
                self.internal_boolean_ops(|a, b| a < b, |a, b| a < b, "<", lhs, rhs)?;
            }
            InstOp::Lte => {
                self.internal_boolean_ops(|a, b| a <= b, |a, b| a <= b, "<=", lhs, rhs)?;
            }
            InstOp::Eq => {
                let rhs = self.stack.address(rhs)?;
                let lhs = self.stack.address(lhs)?;
                let test = Value::value_ptr_eq(self, &lhs, &rhs)?;
                self.stack.push(test);
            }
            InstOp::Neq => {
                let rhs = self.stack.address(rhs)?;
                let lhs = self.stack.address(lhs)?;
                let test = Value::value_ptr_eq(self, &lhs, &rhs)?;
                self.stack.push(!test);
            }
            InstOp::And => {
                self.internal_boolean_op(|a, b| a && b, "&&", lhs, rhs)?;
            }
            InstOp::Or => {
                self.internal_boolean_op(|a, b| a || b, "||", lhs, rhs)?;
            }
            InstOp::Is => {
                let is_instance = self.is_instance(lhs, rhs)?;
                self.stack.push(is_instance);
            }
            InstOp::IsNot => {
                let is_instance = self.is_instance(lhs, rhs)?;
                self.stack.push(!is_instance);
            }
        }

        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_assign(&mut self, target: InstTarget, op: InstAssignOp) -> Result<(), VmError> {
        use std::convert::TryFrom as _;

        match op {
            InstAssignOp::Add => {
                self.internal_num_assign(
                    target,
                    Protocol::ADD_ASSIGN,
                    || VmErrorKind::Overflow,
                    i64::checked_add,
                    std::ops::Add::add,
                )?;
            }
            InstAssignOp::Sub => {
                self.internal_num_assign(
                    target,
                    Protocol::SUB_ASSIGN,
                    || VmErrorKind::Underflow,
                    i64::checked_sub,
                    std::ops::Sub::sub,
                )?;
            }
            InstAssignOp::Mul => {
                self.internal_num_assign(
                    target,
                    Protocol::MUL_ASSIGN,
                    || VmErrorKind::Overflow,
                    i64::checked_mul,
                    std::ops::Mul::mul,
                )?;
            }
            InstAssignOp::Div => {
                self.internal_num_assign(
                    target,
                    Protocol::DIV_ASSIGN,
                    || VmErrorKind::DivideByZero,
                    i64::checked_div,
                    std::ops::Div::div,
                )?;
            }
            InstAssignOp::Rem => {
                self.internal_num_assign(
                    target,
                    Protocol::REM_ASSIGN,
                    || VmErrorKind::DivideByZero,
                    i64::checked_rem,
                    std::ops::Rem::rem,
                )?;
            }
            InstAssignOp::BitAnd => {
                self.internal_infallible_bitwise_assign(
                    target,
                    Protocol::BIT_AND_ASSIGN,
                    std::ops::BitAndAssign::bitand_assign,
                )?;
            }
            InstAssignOp::BitXor => {
                self.internal_infallible_bitwise_assign(
                    target,
                    Protocol::BIT_XOR_ASSIGN,
                    std::ops::BitXorAssign::bitxor_assign,
                )?;
            }
            InstAssignOp::BitOr => {
                self.internal_infallible_bitwise_assign(
                    target,
                    Protocol::BIT_OR_ASSIGN,
                    std::ops::BitOrAssign::bitor_assign,
                )?;
            }
            InstAssignOp::Shl => {
                self.internal_bitwise_assign(
                    target,
                    Protocol::SHL_ASSIGN,
                    || VmErrorKind::Overflow,
                    |a, b| a.checked_shl(u32::try_from(b).ok()?),
                )?;
            }
            InstAssignOp::Shr => {
                self.internal_infallible_bitwise_assign(
                    target,
                    Protocol::SHR_ASSIGN,
                    std::ops::ShrAssign::shr_assign,
                )?;
            }
        }

        Ok(())
    }

    /// Perform an index set operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_index_set(&mut self) -> Result<(), VmError> {
        let index = self.stack.pop()?;
        let target = self.stack.pop()?;
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
                Value::Struct(typed_object) => {
                    let mut typed_object = typed_object.borrow_mut()?;

                    if let Some(v) = typed_object.get_mut(field) {
                        *v = value;
                        return Ok(());
                    }

                    return Err(VmError::from(VmErrorKind::MissingField {
                        field: field.to_owned(),
                        target: typed_object.type_info(),
                    }));
                }
                Value::Variant(variant) => {
                    let mut variant = variant.borrow_mut()?;

                    if let VariantData::Struct(st) = variant.data_mut() {
                        if let Some(v) = st.get_mut(field) {
                            *v = value;
                            return Ok(());
                        }
                    }

                    return Err(VmError::from(VmErrorKind::MissingField {
                        field: field.to_owned(),
                        target: variant.type_info(),
                    }));
                }
                _ => {
                    break;
                }
            }
        }

        if !self.call_instance_fn(target.clone(), Protocol::INDEX_SET, (&index, &value))? {
            return Err(VmError::from(VmErrorKind::UnsupportedIndexSet {
                target: target.type_info()?,
                index: index.type_info()?,
                value: value.type_info()?,
            }));
        }

        // Calling index set should not produce a value on the stack, but all
        // handler functions to produce a value. So pop it here.
        self.stack.pop()?;
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_return(&mut self) -> Result<bool, VmError> {
        let return_value = self.stack.pop()?;
        let exit = self.pop_call_frame()?;
        self.stack.push(return_value);
        Ok(exit)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_return_unit(&mut self) -> Result<bool, VmError> {
        let exit = self.pop_call_frame()?;
        self.stack.push(());
        Ok(exit)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_load_instance_fn(&mut self, hash: Hash) -> Result<(), VmError> {
        let instance = self.stack.pop()?;
        let ty = instance.type_hash()?;
        let hash = Hash::instance_function(ty, hash);
        self.stack.push(Value::Type(hash));
        Ok(())
    }

    /// Perform an index get operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_index_get(&mut self, target: InstAddress, index: InstAddress) -> Result<(), VmError> {
        let index = self.stack.address(index)?;
        let target = self.stack.address_ref(target)?;

        match &index {
            Value::String(string) => {
                let string_ref = string.borrow_ref()?;

                if let Some(value) = Self::try_object_like_index_get(&target, string_ref.as_str())?
                {
                    self.stack.push(value);
                    return Ok(());
                }
            }
            Value::StaticString(string) => {
                if let Some(value) = Self::try_object_like_index_get(&target, string.as_ref())? {
                    self.stack.push(value);
                    return Ok(());
                }
            }
            Value::Integer(index) => {
                use std::convert::TryInto as _;

                let index = match (*index).try_into() {
                    Ok(index) => index,
                    Err(..) => {
                        return Err(VmError::from(VmErrorKind::MissingIndex {
                            target: target.type_info()?,
                            index: VmIntegerRepr::from(*index),
                        }));
                    }
                };

                if let Some(value) = Self::try_tuple_like_index_get(&target, index)? {
                    self.stack.push(value);
                    return Ok(());
                }
            }
            _ => (),
        }

        let target = target.into_owned();

        if !self.call_instance_fn(target.clone(), Protocol::INDEX_GET, (&index,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedIndexGet {
                target: target.type_info()?,
                index: index.type_info()?,
            }));
        }

        Ok(())
    }

    /// Perform an index get operation specialized for tuples.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_index_get(&mut self, index: usize) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        if let Some(value) = Self::try_tuple_like_index_get(&value, index)? {
            self.stack.push(value);
            return Ok(());
        }

        Err(VmError::from(VmErrorKind::UnsupportedTupleIndexGet {
            target: value.type_info()?,
        }))
    }

    /// Perform an index get operation specialized for tuples.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_index_set(&mut self, index: usize) -> Result<(), VmError> {
        let tuple = self.stack.pop()?;
        let value = self.stack.pop()?;

        if Self::try_tuple_like_index_set(&tuple, index, value)? {
            return Ok(());
        }

        Err(VmError::from(VmErrorKind::UnsupportedTupleIndexSet {
            target: tuple.type_info()?,
        }))
    }

    /// Perform an index get operation specialized for tuples.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_index_get_at(&mut self, offset: usize, index: usize) -> Result<(), VmError> {
        let value = self.stack.at_offset(offset)?;

        if let Some(value) = Self::try_tuple_like_index_get(value, index)? {
            self.stack.push(value);
            return Ok(());
        }

        Err(VmError::from(VmErrorKind::UnsupportedTupleIndexGet {
            target: value.type_info()?,
        }))
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_bool(&mut self, boolean: bool) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.stack.push(match value {
            Value::Bool(actual) => actual == boolean,
            _ => false,
        });

        Ok(())
    }

    /// Perform a specialized index get operation on an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object_index_get(&mut self, string_slot: usize) -> Result<(), VmError> {
        let target = self.stack.pop()?;

        if let Some(value) = self.try_object_slot_index_get(&target, string_slot)? {
            self.stack.push(value);
            return Ok(());
        }

        let target = target.type_info()?;
        Err(VmError::from(VmErrorKind::UnsupportedObjectSlotIndexGet {
            target,
        }))
    }

    /// Perform a specialized index set operation on an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object_index_set(&mut self, string_slot: usize) -> Result<(), VmError> {
        let target = self.stack.pop()?;
        let value = self.stack.pop()?;

        if let Some(()) = self.try_object_slot_index_set(&target, string_slot, value)? {
            return Ok(());
        }

        let target = target.type_info()?;
        Err(VmError::from(VmErrorKind::UnsupportedObjectSlotIndexSet {
            target,
        }))
    }

    /// Perform a specialized index get operation on an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object_index_get_at(&mut self, offset: usize, string_slot: usize) -> Result<(), VmError> {
        let target = self.stack.at_offset(offset)?.clone();

        if let Some(value) = self.try_object_slot_index_get(&target, string_slot)? {
            self.stack.push(value);
            return Ok(());
        }

        let target = target.type_info()?;
        Err(VmError::from(VmErrorKind::UnsupportedObjectSlotIndexGet {
            target,
        }))
    }

    /// Operation to allocate an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object(&mut self, slot: usize) -> Result<(), VmError> {
        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or(VmErrorKind::MissingStaticObjectKeys { slot })?;

        let mut object = Object::with_capacity(keys.len());
        let values = self.stack.drain_stack_top(keys.len())?;

        for (key, value) in keys.iter().zip(values) {
            object.insert(key.clone(), value);
        }

        self.stack.push(Shared::new(object));
        Ok(())
    }

    /// Operation to allocate an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_range(&mut self, limits: InstRangeLimits) -> Result<(), VmError> {
        let end = Option::<Value>::from_value(self.stack.pop()?)?;
        let start = Option::<Value>::from_value(self.stack.pop()?)?;

        let limits = match limits {
            InstRangeLimits::HalfOpen => RangeLimits::HalfOpen,
            InstRangeLimits::Closed => RangeLimits::Closed,
        };

        let range = Range::new(start, end, limits);
        self.stack.push(Shared::new(range));
        Ok(())
    }

    /// Operation to allocate an empty struct.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_empty_struct(&mut self, hash: Hash) -> Result<(), VmError> {
        let rtti = self
            .unit
            .lookup_rtti(hash)
            .ok_or(VmErrorKind::MissingRtti { hash })?;

        self.stack.push(UnitStruct { rtti: rtti.clone() });
        Ok(())
    }

    /// Operation to allocate an object struct.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_struct(&mut self, hash: Hash, slot: usize) -> Result<(), VmError> {
        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or(VmErrorKind::MissingStaticObjectKeys { slot })?;

        let rtti = self
            .unit
            .lookup_rtti(hash)
            .ok_or(VmErrorKind::MissingRtti { hash })?;

        let values = self.stack.drain_stack_top(keys.len())?;
        let mut data = Object::with_capacity(keys.len());

        for (key, value) in keys.iter().zip(values) {
            data.insert(key.clone(), value);
        }

        self.stack.push(Struct {
            rtti: rtti.clone(),
            data,
        });

        Ok(())
    }

    /// Operation to allocate an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_unit_variant(&mut self, hash: Hash) -> Result<(), VmError> {
        let rtti = self
            .unit
            .lookup_variant_rtti(hash)
            .ok_or(VmErrorKind::MissingVariantRtti { hash })?;

        self.stack.push(Variant::unit(rtti.clone()));
        Ok(())
    }

    /// Operation to allocate an object variant.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object_variant(&mut self, hash: Hash, slot: usize) -> Result<(), VmError> {
        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or(VmErrorKind::MissingStaticObjectKeys { slot })?;

        let rtti = self
            .unit
            .lookup_variant_rtti(hash)
            .ok_or(VmErrorKind::MissingVariantRtti { hash })?;

        let mut data = Object::with_capacity(keys.len());
        let values = self.stack.drain_stack_top(keys.len())?;

        for (key, value) in keys.iter().zip(values) {
            data.insert(key.clone(), value);
        }

        self.stack.push(Variant::struct_(rtti.clone(), data));
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_string(&mut self, slot: usize) -> Result<(), VmError> {
        let string = self.unit.lookup_string(slot)?;
        self.stack.push(string.clone());
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_bytes(&mut self, slot: usize) -> Result<(), VmError> {
        let bytes = self.unit.lookup_bytes(slot)?.to_owned();
        self.stack.push(Bytes::from_vec(bytes));
        Ok(())
    }

    /// Optimize operation to perform string concatenation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_string_concat(&mut self, len: usize, size_hint: usize) -> Result<(), VmError> {
        let values = self.stack.drain_stack_top(len)?.collect::<vec::Vec<_>>();

        let mut out = String::with_capacity(size_hint);
        let mut buf = String::with_capacity(16);

        for value in values {
            if let Err(fmt::Error) = value.string_display_with(&mut out, &mut buf, &mut *self)? {
                return Err(VmError::from(VmErrorKind::FormatError));
            }
        }

        self.stack.push(out);
        Ok(())
    }

    /// Push a format specification onto the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_format(&mut self, spec: FormatSpec) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        self.stack.push(Format { value, spec });
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_unwrap(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let value = match value {
            Value::Option(option) => match &*option.borrow_ref()? {
                Some(value) => value.clone(),
                None => {
                    return Err(VmError::from(VmErrorKind::UnsupportedUnwrapNone));
                }
            },
            Value::Result(result) => match &*result.borrow_ref()? {
                Ok(value) => value.clone(),
                Err(err) => {
                    return Err(VmError::from(VmErrorKind::UnsupportedUnwrapErr {
                        err: err.type_info()?,
                    }));
                }
            },
            other => {
                return Err(VmError::from(VmErrorKind::UnsupportedUnwrap {
                    actual: other.type_info()?,
                }));
            }
        };

        self.stack.push(value);
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_is_unit(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        self.stack.push(matches!(value, Value::Unit));
        Ok(())
    }

    /// Test if the top of the stack is an error.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_is_value(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let is_value = match value {
            Value::Result(result) => result.borrow_ref()?.is_ok(),
            Value::Option(option) => option.borrow_ref()?.is_some(),
            other => {
                return Err(VmError::from(VmErrorKind::UnsupportedIsValueOperand {
                    actual: other.type_info()?,
                }))
            }
        };

        self.stack.push(is_value);
        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_byte(&mut self, byte: u8) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.stack.push(match value {
            Value::Byte(actual) => actual == byte,
            _ => false,
        });

        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_character(&mut self, character: char) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.stack.push(match value {
            Value::Char(actual) => actual == character,
            _ => false,
        });

        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_integer(&mut self, integer: i64) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.stack.push(match value {
            Value::Integer(actual) => actual == integer,
            _ => false,
        });

        Ok(())
    }

    /// Test if the top of stack is equal to the string at the given static
    /// string location.
    #[cfg_attr(feature = "bench", inline(never))]
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

    #[cfg_attr(feature = "bench", inline(never))]
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

    #[cfg_attr(feature = "bench", inline(never))]
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

    /// Push the given variant onto the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_variant(&mut self, variant: InstVariant) -> Result<(), VmError> {
        match variant {
            InstVariant::Some => {
                let some = self.stack.pop()?;
                self.stack.push(Value::Option(Shared::new(Some(some))));
            }
            InstVariant::None => {
                self.stack.push(Value::Option(Shared::new(None)));
            }
            InstVariant::Ok => {
                let some = self.stack.pop()?;
                self.stack.push(Value::Result(Shared::new(Ok(some))));
            }
            InstVariant::Err => {
                let some = self.stack.pop()?;
                self.stack.push(Value::Result(Shared::new(Err(some))));
            }
        }

        Ok(())
    }

    /// Load a function as a value onto the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_load_fn(&mut self, hash: Hash) -> Result<(), VmError> {
        let function = match self.unit.lookup(hash) {
            Some(info) => match info {
                UnitFn::Offset { offset, call, args } => Function::from_offset(
                    self.context.clone(),
                    self.unit.clone(),
                    offset,
                    call,
                    args,
                    hash,
                ),
                UnitFn::UnitStruct { hash } => {
                    let rtti = self
                        .unit
                        .lookup_rtti(hash)
                        .ok_or(VmErrorKind::MissingRtti { hash })?;

                    Function::from_unit_struct(rtti.clone())
                }
                UnitFn::TupleStruct { hash, args } => {
                    let rtti = self
                        .unit
                        .lookup_rtti(hash)
                        .ok_or(VmErrorKind::MissingRtti { hash })?;

                    Function::from_tuple_struct(rtti.clone(), args)
                }
                UnitFn::UnitVariant { hash } => {
                    let rtti = self
                        .unit
                        .lookup_variant_rtti(hash)
                        .ok_or(VmErrorKind::MissingVariantRtti { hash })?;

                    Function::from_unit_variant(rtti.clone())
                }
                UnitFn::TupleVariant { hash, args } => {
                    let rtti = self
                        .unit
                        .lookup_variant_rtti(hash)
                        .ok_or(VmErrorKind::MissingVariantRtti { hash })?;

                    Function::from_tuple_variant(rtti.clone(), args)
                }
            },
            None => {
                let handler = self
                    .context
                    .lookup(hash)
                    .ok_or(VmErrorKind::MissingFunction { hash })?;

                Function::from_handler(handler.clone(), hash)
            }
        };

        self.stack.push(Value::Function(Shared::new(function)));
        Ok(())
    }

    /// Construct a closure on the top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_closure(&mut self, hash: Hash, count: usize) -> Result<(), VmError> {
        let info = self
            .unit
            .lookup(hash)
            .ok_or(VmErrorKind::MissingFunction { hash })?;

        let (offset, call, args) = match info {
            UnitFn::Offset { offset, call, args } => (offset, call, args),
            _ => return Err(VmError::from(VmErrorKind::MissingFunction { hash })),
        };

        let environment = self.stack.pop_sequence(count)?.into_boxed_slice();

        let function = Function::from_closure(
            self.context.clone(),
            self.unit.clone(),
            offset,
            call,
            args,
            environment,
            hash,
        );

        self.stack.push(Value::Function(Shared::new(function)));
        Ok(())
    }

    /// Implementation of a function call.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_call(&mut self, hash: Hash, args: usize) -> Result<(), VmError> {
        match self.unit.lookup(hash) {
            Some(info) => match info {
                UnitFn::Offset {
                    offset,
                    call,
                    args: expected,
                } => {
                    Self::check_args(args, expected)?;
                    self.call_offset_fn(offset, call, args)?;
                }
                UnitFn::UnitStruct { hash } => {
                    Self::check_args(args, 0)?;

                    let rtti = self
                        .unit
                        .lookup_rtti(hash)
                        .ok_or(VmErrorKind::MissingRtti { hash })?;

                    self.stack.push(Value::unit_struct(rtti.clone()));
                }
                UnitFn::TupleStruct {
                    hash,
                    args: expected,
                } => {
                    Self::check_args(args, expected)?;
                    let tuple = self.stack.pop_sequence(args)?;

                    let rtti = self
                        .unit
                        .lookup_rtti(hash)
                        .ok_or(VmErrorKind::MissingRtti { hash })?;

                    self.stack.push(Value::tuple_struct(rtti.clone(), tuple));
                }
                UnitFn::TupleVariant {
                    hash,
                    args: expected,
                } => {
                    Self::check_args(args, expected)?;

                    let rtti = self
                        .unit
                        .lookup_variant_rtti(hash)
                        .ok_or(VmErrorKind::MissingVariantRtti { hash })?;

                    let tuple = self.stack.pop_sequence(args)?;
                    self.stack.push(Value::tuple_variant(rtti.clone(), tuple));
                }
                UnitFn::UnitVariant { hash } => {
                    Self::check_args(args, 0)?;

                    let rtti = self
                        .unit
                        .lookup_variant_rtti(hash)
                        .ok_or(VmErrorKind::MissingVariantRtti { hash })?;

                    self.stack.push(Value::unit_variant(rtti.clone()));
                }
            },
            None => {
                let handler = self
                    .context
                    .lookup(hash)
                    .ok_or(VmErrorKind::MissingFunction { hash })?;

                handler(&mut self.stack, args)?;
            }
        }

        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_call_instance(
        &mut self,
        inst_fn: impl InstFnNameHash,
        args: usize,
    ) -> Result<(), VmError> {
        self.inner_op_call_instance(inst_fn.inst_fn_name_hash(), args)
    }

    #[inline(never)]
    fn inner_op_call_instance(&mut self, inst_fn: Hash, args: usize) -> Result<(), VmError> {
        // NB: +1 to include the instance itself.
        let args = args + 1;
        let instance = self.stack.at_offset_from_top(args)?;
        let type_hash = instance.type_hash()?;
        let hash = Hash::instance_function(type_hash, inst_fn);

        match self.unit.lookup(hash) {
            Some(info) => match info {
                UnitFn::Offset {
                    offset,
                    call,
                    args: expected,
                } => {
                    Self::check_args(args, expected)?;
                    self.call_offset_fn(offset, call, args)?;
                }
                _ => {
                    return Err(VmError::from(VmErrorKind::MissingInstanceFunction {
                        instance: instance.type_info()?,
                        hash,
                    }));
                }
            },
            None => {
                let handler = match self.context.lookup(hash) {
                    Some(handler) => handler,
                    None => {
                        return Err(VmError::from(VmErrorKind::MissingInstanceFunction {
                            instance: instance.type_info()?,
                            hash,
                        }));
                    }
                };

                handler(&mut self.stack, args)?;
            }
        }

        Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_call_fn(&mut self, args: usize) -> Result<Option<VmHalt>, VmError> {
        let function = self.stack.pop()?;

        let hash = match function {
            Value::Type(hash) => hash,
            Value::Function(function) => {
                let function = function.into_ref()?;
                return function.call_with_vm(self, args);
            }
            actual => {
                let actual_type = actual.type_info()?;
                return Err(VmError::from(VmErrorKind::UnsupportedCallFn {
                    actual_type,
                }));
            }
        };

        self.op_call(hash, args)?;
        Ok(None)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_iter_next(&mut self, offset: usize, jump: isize) -> Result<(), VmError> {
        let value = self.stack.at_offset_mut(offset)?;

        let some = match value {
            Value::Option(option) => {
                let option = option.borrow_ref()?.clone();

                match option {
                    Some(some) => some,
                    None => {
                        self.modify_ip(jump)?;
                        return Ok(());
                    }
                }
            }
            other => {
                return Err(VmError::from(VmErrorKind::UnsupportedIsValueOperand {
                    actual: other.type_info()?,
                }))
            }
        };

        *value = some;
        Ok(())
    }

    /// Evaluate a single instruction.
    pub(crate) fn run(&mut self) -> Result<VmHalt, VmError> {
        // NB: set up environment so that native function can access context and
        // unit.
        let _guard = crate::env::Guard::new(&self.context, &self.unit);

        loop {
            if !budget::take() {
                return Ok(VmHalt::Limited);
            }

            let inst = *self
                .unit
                .instruction_at(self.ip)
                .ok_or(VmErrorKind::IpOutOfBounds)?;

            log::trace!("{}: {}", self.ip, inst);

            match inst {
                Inst::Not => {
                    self.op_not()?;
                }
                Inst::Neg => {
                    self.op_neg()?;
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
                    if let Some(reason) = self.op_call_fn(args)? {
                        return Ok(reason);
                    }
                }
                Inst::LoadInstanceFn { hash } => {
                    self.op_load_instance_fn(hash)?;
                }
                Inst::IndexGet { target, index } => {
                    self.op_index_get(target, index)?;
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
                Inst::ObjectIndexGet { slot } => {
                    self.op_object_index_get(slot)?;
                }
                Inst::ObjectIndexSet { slot } => {
                    self.op_object_index_set(slot)?;
                }
                Inst::ObjectIndexGetAt { offset, slot } => {
                    self.op_object_index_get_at(offset, slot)?;
                }
                Inst::IndexSet => {
                    self.op_index_set()?;
                }
                Inst::Return => {
                    if self.op_return()? {
                        self.advance();
                        return Ok(VmHalt::Exited);
                    }
                }
                Inst::ReturnUnit => {
                    if self.op_return_unit()? {
                        self.advance();
                        return Ok(VmHalt::Exited);
                    }
                }
                Inst::Await => {
                    let future = self.op_await()?;
                    // NB: the future itself will advance the virtual machine.
                    return Ok(VmHalt::Awaited(Awaited::Future(future)));
                }
                Inst::Select { len } => {
                    if let Some(select) = self.op_select(len)? {
                        // NB: the future itself will advance the virtual machine.
                        return Ok(VmHalt::Awaited(Awaited::Select(select)));
                    }
                }
                Inst::LoadFn { hash } => {
                    self.op_load_fn(hash)?;
                }
                Inst::Push { value } => {
                    self.op_push(value)?;
                }
                Inst::Pop => {
                    self.op_pop()?;
                }
                Inst::PopN { count } => {
                    self.op_popn(count)?;
                }
                Inst::PopAndJumpIfNot { count, offset } => {
                    self.op_pop_and_jump_if_not(count, offset)?;
                }
                Inst::Clean { count } => {
                    self.op_clean(count)?;
                }
                Inst::Copy { offset } => {
                    self.op_copy(offset)?;
                }
                Inst::Move { offset } => {
                    self.op_move(offset)?;
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
                Inst::Jump { offset } => {
                    self.op_jump(offset)?;
                }
                Inst::JumpIf { offset } => {
                    self.op_jump_if(offset)?;
                }
                Inst::JumpIfOrPop { offset } => {
                    self.op_jump_if_or_pop(offset)?;
                }
                Inst::JumpIfNotOrPop { offset } => {
                    self.op_jump_if_not_or_pop(offset)?;
                }
                Inst::JumpIfBranch { branch, offset } => {
                    self.op_jump_if_branch(branch, offset)?;
                }
                Inst::Vec { count } => {
                    self.op_vec(count)?;
                }
                Inst::Tuple { count } => {
                    self.op_tuple(count)?;
                }
                Inst::Tuple1 { args } => {
                    self.op_tuple_n(&args[..])?;
                }
                Inst::Tuple2 { args } => {
                    self.op_tuple_n(&args[..])?;
                }
                Inst::Tuple3 { args } => {
                    self.op_tuple_n(&args[..])?;
                }
                Inst::Tuple4 { args } => {
                    self.op_tuple_n(&args[..])?;
                }
                Inst::PushTuple => {
                    self.op_push_tuple()?;
                }
                Inst::Object { slot } => {
                    self.op_object(slot)?;
                }
                Inst::Range { limits } => {
                    self.op_range(limits)?;
                }
                Inst::UnitStruct { hash } => {
                    self.op_empty_struct(hash)?;
                }
                Inst::Struct { hash, slot } => {
                    self.op_struct(hash, slot)?;
                }
                Inst::UnitVariant { hash } => {
                    self.op_unit_variant(hash)?;
                }
                Inst::StructVariant { hash, slot } => {
                    self.op_object_variant(hash, slot)?;
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
                Inst::Format { spec } => {
                    self.op_format(spec)?;
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
                Inst::EqByte { byte } => {
                    self.op_eq_byte(byte)?;
                }
                Inst::EqCharacter { character } => {
                    self.op_eq_character(character)?;
                }
                Inst::EqInteger { integer } => {
                    self.op_eq_integer(integer)?;
                }
                Inst::EqBool { boolean } => {
                    self.op_eq_bool(boolean)?;
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
                    return Ok(VmHalt::Yielded);
                }
                Inst::YieldUnit => {
                    self.advance();
                    self.stack.push(Value::Unit);
                    return Ok(VmHalt::Yielded);
                }
                Inst::Variant { variant } => {
                    self.op_variant(variant)?;
                }
                Inst::Op { op, a, b } => {
                    self.op_op(op, a, b)?;
                }
                Inst::Assign { target, op } => {
                    self.op_assign(target, op)?;
                }
                Inst::IterNext { offset, jump } => {
                    self.op_iter_next(offset, jump)?;
                }
                Inst::Panic { reason } => {
                    return Err(VmError::from(VmErrorKind::Panic {
                        reason: Panic::from(reason),
                    }));
                }
            }

            self.advance();
        }
    }
}

/// A call frame.
///
/// This is used to store the return point after an instruction has been run.
#[derive(Debug, Clone, Copy)]
pub struct CallFrame {
    /// The stored instruction pointer.
    ip: usize,
    /// The top of the stack at the time of the call to ensure stack isolation
    /// across function calls.
    ///
    /// I.e. a function should not be able to manipulate the size of any other
    /// stack than its own.
    stack_bottom: usize,
}

impl CallFrame {
    /// Get the instruction pointer of the call frame.
    pub fn ip(&self) -> usize {
        self.ip
    }

    /// Get the bottom of the stack of the current call frame.
    pub fn stack_bottom(&self) -> usize {
        self.stack_bottom
    }
}
