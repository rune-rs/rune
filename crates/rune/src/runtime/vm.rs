use core::fmt;
use core::mem;
use core::ops;
use core::slice;

use crate::hash::{Hash, IntoHash, ToTypeHash};
use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;
use crate::no_std::vec;
use crate::runtime::budget;
use crate::runtime::future::SelectFuture;
use crate::runtime::unit::{UnitFn, UnitStorage};
use crate::runtime::{
    Args, Awaited, BorrowMut, Bytes, Call, Format, FormatSpec, FromValue, Function, Future,
    Generator, GuardedArgs, Inst, InstAddress, InstAssignOp, InstOp, InstRangeLimits, InstTarget,
    InstValue, InstVariant, Object, Panic, Protocol, Range, RangeLimits, RuntimeContext, Select,
    Shared, Stack, Stream, Struct, Tuple, Type, TypeCheck, Unit, UnitStruct, Value, Variant,
    VariantData, Vec, VmError, VmErrorKind, VmExecution, VmHalt, VmIntegerRepr, VmResult,
    VmSendExecution,
};

/// Small helper function to build errors.
fn err<T, E>(error: E) -> VmResult<T>
where
    VmErrorKind: From<E>,
{
    VmResult::err(error)
}

/// The result from a dynamic call. Indicates if the attempted operation is
/// supported.
#[derive(Debug)]
pub(crate) enum CallResult<T> {
    /// Call successful. Return value is on the stack.
    Ok(T),
    /// Call failed because function was missing so the method is unsupported.
    /// Contains target value.
    Unsupported(Value),
}

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
        let rhs = vm_try!($vm.stack.pop());

        match $target {
            InstTarget::Offset(offset) => {
                TargetValue::Value(vm_try!($vm.stack.at_offset_mut(offset)), rhs)
            }
            InstTarget::TupleField(index) => {
                $lhs = vm_try!($vm.stack.pop());

                if let Some(value) = vm_try!(Vm::try_tuple_like_index_get_mut(&$lhs, index)) {
                    $guard = value;
                    TargetValue::Value(&mut *$guard, rhs)
                } else {
                    TargetValue::Fallback(TargetFallback::Index(&$lhs, index, rhs))
                }
            }
            InstTarget::Field(field) => {
                let field = vm_try!($vm.unit.lookup_string(field));
                $lhs = vm_try!($vm.stack.pop());

                if let Some(value) = vm_try!(Vm::try_object_like_index_get_mut(&$lhs, field)) {
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
    context: Arc<RuntimeContext>,
    /// Unit associated with virtual machine.
    unit: Arc<Unit>,
    /// The current instruction pointer.
    ip: usize,
    /// The current stack.
    stack: Stack,
    /// Frames relative to the stack.
    call_frames: vec::Vec<CallFrame>,
}

impl Vm {
    /// Construct a new virtual machine.
    pub const fn new(context: Arc<RuntimeContext>, unit: Arc<Unit>) -> Self {
        Self::with_stack(context, unit, Stack::new())
    }

    /// Construct a new virtual machine with a custom stack.
    pub const fn with_stack(context: Arc<RuntimeContext>, unit: Arc<Unit>, stack: Stack) -> Self {
        Self {
            context,
            unit,
            ip: 0,
            stack,
            call_frames: vec::Vec::new(),
        }
    }

    /// Construct a vm with a default empty [RuntimeContext]. This is useful
    /// when the [Unit] was constructed with an empty
    /// [Context][crate::compile::Context].
    pub fn without_runtime(unit: Arc<Unit>) -> Self {
        Self::new(Default::default(), unit)
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

    /// Reset this virtual machine, freeing all memory used.
    pub fn clear(&mut self) {
        self.ip = 0;
        self.stack.clear();
        self.call_frames.clear();
    }

    /// Look up a function in the virtual machine by its name.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Context, Vm, Unit};
    /// use rune::compile::ItemBuf;
    ///
    /// use std::sync::Arc;
    ///
    /// let context = Context::with_default_modules()?;
    /// let context = Arc::new(context.runtime());
    ///
    /// let mut sources = rune::sources! {
    ///     entry => {
    ///         pub fn max(a, b) {
    ///             if a > b {
    ///                 a
    ///             } else {
    ///                 b
    ///             }
    ///         }
    ///     }
    /// };
    ///
    /// let unit = rune::prepare(&mut sources).build()?;
    /// let unit = Arc::new(unit);
    ///
    /// let vm = Vm::new(context, unit);
    ///
    /// // Looking up an item from the source.
    /// let dynamic_max = vm.lookup_function(["max"])?;
    ///
    /// let value: i64 = rune::from_value(dynamic_max.call((10, 20)).into_result()?)?;
    /// assert_eq!(value, 20);
    ///
    /// // Building an item buffer to lookup an `::std` item.
    /// let mut item = ItemBuf::with_crate("std");
    /// item.push("int");
    /// item.push("max");
    ///
    /// let max = vm.lookup_function(&item)?;
    ///
    /// let value: i64 = rune::from_value(max.call((10, 20)).into_result()?)?;
    /// assert_eq!(value, 20);
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn lookup_function<N>(&self, name: N) -> Result<Function, VmError>
    where
        N: ToTypeHash,
    {
        Ok(self.lookup_function_by_hash(name.to_type_hash())?)
    }

    /// Convert into an execution.
    pub(crate) fn into_execution(self) -> VmExecution<Self> {
        VmExecution::new(self)
    }

    /// Run the given vm to completion.
    ///
    /// If any async instructions are encountered, this will error.
    pub fn complete(self) -> Result<Value, VmError> {
        self.into_execution().complete().into_result()
    }

    /// Run the given vm to completion with support for async functions.
    pub async fn async_complete(self) -> Result<Value, VmError> {
        self.into_execution().async_complete().await.into_result()
    }

    /// Call the function identified by the given name.
    ///
    /// Computing the function hash from the name can be a bit costly, so it's
    /// worth noting that it can be precalculated:
    ///
    /// ```
    /// use rune::Hash;
    ///
    /// let name = Hash::type_hash(["main"]);
    /// ```
    ///
    /// # Examples
    ///
    /// ```,no_run
    /// use rune::{Context, Unit};
    /// use std::sync::Arc;
    ///
    /// let context = Context::with_default_modules()?;
    /// let context = Arc::new(context.runtime());
    ///
    /// // Normally the unit would be created by compiling some source,
    /// // and since this one is empty it won't do anything.
    /// let unit = Arc::new(Unit::default());
    ///
    /// let mut vm = rune::Vm::new(context, unit);
    ///
    /// let output = vm.execute(["main"], (33i64,))?.complete().into_result()?;
    /// let output: i64 = rune::from_value(output)?;
    ///
    /// println!("output: {}", output);
    /// # Ok::<_, rune::Error>(())
    /// ```
    ///
    /// You can use a `Vec<Value>` to provide a variadic collection of
    /// arguments.
    ///
    /// ```,no_run
    /// use rune::{Context, Unit};
    /// use std::sync::Arc;
    ///
    /// let context = Context::with_default_modules()?;
    /// let context = Arc::new(context.runtime());
    ///
    /// // Normally the unit would be created by compiling some source,
    /// // and since this one is empty it won't do anything.
    /// let unit = Arc::new(Unit::default());
    ///
    /// let mut vm = rune::Vm::new(context, unit);
    ///
    /// let mut args = Vec::new();
    /// args.push(rune::to_value(1u32)?);
    /// args.push(rune::to_value(String::from("Hello World"))?);
    ///
    /// let output = vm.execute(["main"], args)?.complete().into_result()?;
    /// let output: i64 = rune::from_value(output)?;
    ///
    /// println!("output: {}", output);
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn execute<A, N>(&mut self, name: N, args: A) -> Result<VmExecution<&mut Self>, VmError>
    where
        N: ToTypeHash,
        A: Args,
    {
        self.set_entrypoint(name, args.count())?;
        args.into_stack(&mut self.stack).into_result()?;
        Result::Ok(VmExecution::new(self))
    }

    /// An `execute` variant that returns an execution which implements
    /// [`Send`], allowing it to be sent and executed on a different thread.
    ///
    /// This is accomplished by preventing values escaping from being
    /// non-exclusively sent with the execution or escaping the execution. We
    /// only support encoding arguments which themselves are `Send`.
    pub fn send_execute<A, N>(mut self, name: N, args: A) -> Result<VmSendExecution, VmError>
    where
        N: ToTypeHash,
        A: Send + Args,
    {
        // Safety: make sure the stack is clear, preventing any values from
        // being sent along with the virtual machine.
        self.stack.clear();

        self.set_entrypoint(name, args.count())?;
        args.into_stack(&mut self.stack).into_result()?;
        Result::Ok(VmSendExecution(VmExecution::new(self)))
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
    /// [`Mut<T>`]: crate::runtime::Mut
    /// [`Ref<T>`]: crate::runtime::Ref
    pub fn call<A, N>(&mut self, name: N, args: A) -> Result<Value, VmError>
    where
        N: ToTypeHash,
        A: GuardedArgs,
    {
        self.set_entrypoint(name, args.count())?;

        // Safety: We hold onto the guard until the vm has completed and
        // `VmExecution` will clear the stack before this function returns.
        // Erronously or not.
        let guard = unsafe { args.unsafe_into_stack(&mut self.stack).into_result()? };

        let value = {
            // Clearing the stack here on panics has safety implications - see
            // above.
            let vm = ClearStack(self);
            VmExecution::new(&mut *vm.0).complete().into_result()?
        };

        // Note: this might panic if something in the vm is holding on to a
        // reference of the value. We should prevent it from being possible to
        // take any owned references to values held by this.
        drop(guard);
        Result::Ok(value)
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
    /// [`Mut<T>`]: crate::runtime::Mut
    /// [`Ref<T>`]: crate::runtime::Ref
    pub async fn async_call<A, N>(&mut self, name: N, args: A) -> Result<Value, VmError>
    where
        N: ToTypeHash,
        A: GuardedArgs,
    {
        self.set_entrypoint(name, args.count())?;

        // Safety: We hold onto the guard until the vm has completed and
        // `VmExecution` will clear the stack before this function returns.
        // Erronously or not.
        let guard = unsafe { args.unsafe_into_stack(&mut self.stack).into_result()? };

        let value = {
            // Clearing the stack here on panics has safety implications - see
            // above.
            let vm = ClearStack(self);
            VmExecution::new(&mut *vm.0)
                .async_complete()
                .await
                .into_result()?
        };

        // Note: this might panic if something in the vm is holding on to a
        // reference of the value. We should prevent it from being possible to
        // take any owned references to values held by this.
        drop(guard);
        Result::Ok(value)
    }

    /// Update the instruction pointer to match the function matching the given
    /// name and check that the number of argument matches.
    fn set_entrypoint<N>(&mut self, name: N, count: usize) -> Result<(), VmErrorKind>
    where
        N: ToTypeHash,
    {
        let hash = name.to_type_hash();

        let info = self.unit.function(hash).ok_or_else(|| {
            if let Some(item) = name.to_item() {
                VmErrorKind::MissingEntry { hash, item }
            } else {
                VmErrorKind::MissingEntryHash { hash }
            }
        })?;

        let offset = match info {
            // NB: we ignore the calling convention.
            // everything is just async when called externally.
            UnitFn::Offset {
                offset,
                args: expected,
                ..
            } => {
                check_args(count, expected)?;
                offset
            }
            _ => {
                return Err(VmErrorKind::MissingFunction { hash });
            }
        };

        self.ip = offset;
        self.stack.clear();
        self.call_frames.clear();
        Ok(())
    }

    /// Helper function to call an instance function.
    #[inline(always)]
    pub(crate) fn call_instance_fn<H, A>(
        &mut self,
        target: Value,
        hash: H,
        args: A,
    ) -> VmResult<CallResult<()>>
    where
        H: ToTypeHash,
        A: GuardedArgs,
    {
        let count = args.count();
        let full_count = args.count() + 1;
        let type_hash = vm_try!(target.type_hash());
        self.stack.push(target);

        // Safety: We hold onto the guard for the duration of this call.
        let _guard = unsafe { vm_try!(args.unsafe_into_stack(&mut self.stack)) };

        let hash = Hash::associated_function(type_hash, hash.to_type_hash());

        if let Some(UnitFn::Offset {
            offset,
            call,
            args: expected,
        }) = self.unit.function(hash)
        {
            vm_try!(check_args(full_count, expected));
            vm_try!(self.call_offset_fn(offset, call, full_count));
            return VmResult::Ok(CallResult::Ok(()));
        }

        if let Some(handler) = self.context.function(hash) {
            vm_try!(handler(&mut self.stack, full_count));
            return VmResult::Ok(CallResult::Ok(()));
        }

        // Restore the stack!
        vm_try!(self.stack.popn(count));
        let target = vm_try!(self.stack.pop());
        VmResult::Ok(CallResult::Unsupported(target))
    }

    /// Helper to call a field function.
    #[inline(always)]
    fn call_field_fn<N, A>(
        &mut self,
        protocol: Protocol,
        target: Value,
        name: N,
        args: A,
    ) -> VmResult<CallResult<()>>
    where
        N: IntoHash,
        A: Args,
    {
        let count = args.count();
        let full_count = count + 1;
        let hash = Hash::field_function(protocol, vm_try!(target.type_hash()), name);

        self.stack.push(target);
        vm_try!(args.into_stack(&mut self.stack));

        if let Some(handler) = self.context.function(hash) {
            vm_try!(handler(&mut self.stack, full_count));
            return VmResult::Ok(CallResult::Ok(()));
        }

        // Restore the stack!
        vm_try!(self.stack.popn(count));
        let target = vm_try!(self.stack.pop());
        VmResult::Ok(CallResult::Unsupported(target))
    }

    /// Helper to call an index function.
    #[inline(always)]
    fn call_index_fn<A>(
        &mut self,
        protocol: Protocol,
        target: Value,
        index: usize,
        args: A,
    ) -> VmResult<CallResult<()>>
    where
        A: Args,
    {
        let count = args.count();
        let full_count = count + 1;
        let hash = Hash::index_function(protocol, vm_try!(target.type_hash()), Hash::index(index));

        self.stack.push(target);
        vm_try!(args.into_stack(&mut self.stack));

        if let Some(handler) = self.context.function(hash) {
            vm_try!(handler(&mut self.stack, full_count));
            return VmResult::Ok(CallResult::Ok(()));
        }

        // Restore the stack!
        vm_try!(self.stack.popn(count));
        let target = vm_try!(self.stack.pop());
        VmResult::Ok(CallResult::Unsupported(target))
    }

    fn internal_boolean_ops(
        &mut self,
        int_op: fn(i64, i64) -> bool,
        float_op: fn(f64, f64) -> bool,
        op: &'static str,
        lhs: InstAddress,
        rhs: InstAddress,
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.address(rhs));
        let lhs = vm_try!(self.stack.address(lhs));

        let out = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => int_op(lhs, rhs),
            (Value::Float(lhs), Value::Float(rhs)) => float_op(lhs, rhs),
            (lhs, rhs) => {
                return err(VmErrorKind::UnsupportedBinaryOperation {
                    op,
                    lhs: vm_try!(lhs.type_info()),
                    rhs: vm_try!(rhs.type_info()),
                });
            }
        };

        self.stack.push(out);
        VmResult::Ok(())
    }

    /// Push a new call frame.
    ///
    /// This will cause the `args` number of elements on the stack to be
    /// associated and accessible to the new call frame.
    pub(crate) fn push_call_frame(&mut self, ip: usize, args: usize) -> Result<(), VmErrorKind> {
        let stack_top = self.stack.swap_stack_bottom(args)?;

        self.call_frames.push(CallFrame {
            ip: self.ip,
            stack_bottom: stack_top,
        });

        self.ip = ip;
        Ok(())
    }

    /// Pop a call frame and return it.
    #[tracing::instrument(skip_all)]
    fn pop_call_frame(&mut self) -> Result<bool, VmErrorKind> {
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
    fn try_object_like_index_get(target: &Value, field: &str) -> VmResult<Option<Value>> {
        let value = match &target {
            Value::Object(target) => vm_try!(target.borrow_ref()).get(field).cloned(),
            Value::Struct(target) => vm_try!(target.borrow_ref()).get(field).cloned(),
            Value::Variant(variant) => match vm_try!(variant.borrow_ref()).data() {
                VariantData::Struct(target) => target.get(field).cloned(),
                _ => return VmResult::Ok(None),
            },
            _ => return VmResult::Ok(None),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return err(VmErrorKind::MissingField {
                    target: vm_try!(target.type_info()),
                    field: field.to_owned(),
                });
            }
        };

        VmResult::Ok(Some(value))
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_tuple_like_index_get(target: &Value, index: usize) -> VmResult<Option<Value>> {
        let value = match target {
            Value::Unit => None,
            Value::Tuple(tuple) => vm_try!(tuple.borrow_ref()).get(index).cloned(),
            Value::Vec(vec) => vm_try!(vec.borrow_ref()).get(index).cloned(),
            Value::Result(result) => {
                let result = vm_try!(result.borrow_ref());

                match &*result {
                    Result::Ok(value) if index == 0 => Some(value.clone()),
                    Result::Err(value) if index == 0 => Some(value.clone()),
                    _ => None,
                }
            }
            Value::Option(option) => {
                let option = vm_try!(option.borrow_ref());

                match &*option {
                    Some(value) if index == 0 => Some(value.clone()),
                    _ => None,
                }
            }
            Value::GeneratorState(state) => {
                use crate::runtime::GeneratorState::*;
                let state = vm_try!(state.borrow_ref());

                match &*state {
                    Yielded(value) if index == 0 => Some(value.clone()),
                    Complete(value) if index == 0 => Some(value.clone()),
                    _ => None,
                }
            }
            Value::TupleStruct(tuple_struct) => {
                let tuple_struct = vm_try!(tuple_struct.borrow_ref());
                tuple_struct.data().get(index).cloned()
            }
            Value::Variant(variant) => {
                let variant = vm_try!(variant.borrow_ref());

                match variant.data() {
                    VariantData::Tuple(tuple) => tuple.get(index).cloned(),
                    _ => return VmResult::Ok(None),
                }
            }
            _ => return VmResult::Ok(None),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return err(VmErrorKind::MissingIndex {
                    target: vm_try!(target.type_info()),
                    index: VmIntegerRepr::from(index),
                });
            }
        };

        VmResult::Ok(Some(value))
    }

    /// Implementation of getting a mutable value out of a tuple-like value.
    fn try_tuple_like_index_get_mut(
        target: &Value,
        index: usize,
    ) -> VmResult<Option<BorrowMut<'_, Value>>> {
        let value = match target {
            Value::Unit => None,
            Value::Tuple(tuple) => {
                let tuple = vm_try!(tuple.borrow_mut());

                BorrowMut::try_map(tuple, |tuple| tuple.get_mut(index))
            }
            Value::Vec(vec) => {
                let vec = vm_try!(vec.borrow_mut());

                BorrowMut::try_map(vec, |vec| vec.get_mut(index))
            }
            Value::Result(result) => {
                let result = vm_try!(result.borrow_mut());

                BorrowMut::try_map(result, |result| match result {
                    Result::Ok(value) if index == 0 => Some(value),
                    Result::Err(value) if index == 0 => Some(value),
                    _ => None,
                })
            }
            Value::Option(option) => {
                let option = vm_try!(option.borrow_mut());

                BorrowMut::try_map(option, |option| match option {
                    Some(value) if index == 0 => Some(value),
                    _ => None,
                })
            }
            Value::GeneratorState(state) => {
                use crate::runtime::GeneratorState::*;
                let state = vm_try!(state.borrow_mut());

                BorrowMut::try_map(state, |state| match state {
                    Yielded(value) if index == 0 => Some(value),
                    Complete(value) if index == 0 => Some(value),
                    _ => None,
                })
            }
            Value::TupleStruct(tuple_struct) => {
                let tuple_struct = vm_try!(tuple_struct.borrow_mut());

                BorrowMut::try_map(tuple_struct, |tuple_struct| tuple_struct.get_mut(index))
            }
            Value::Variant(variant) => {
                let variant = vm_try!(variant.borrow_mut());

                BorrowMut::try_map(variant, |variant| match variant.data_mut() {
                    VariantData::Tuple(tuple) => tuple.get_mut(index),
                    _ => None,
                })
            }
            _ => return VmResult::Ok(None),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return err(VmErrorKind::MissingIndex {
                    target: vm_try!(target.type_info()),
                    index: VmIntegerRepr::from(index),
                });
            }
        };

        VmResult::Ok(Some(value))
    }

    /// Implementation of getting a mutable string index on an object-like type.
    fn try_object_like_index_get_mut<'a>(
        target: &'a Value,
        field: &str,
    ) -> VmResult<Option<BorrowMut<'a, Value>>> {
        let value = match &target {
            Value::Object(target) => {
                let target = vm_try!(target.borrow_mut());
                BorrowMut::try_map(target, |target| target.get_mut(field))
            }
            Value::Struct(target) => {
                let target = vm_try!(target.borrow_mut());
                BorrowMut::try_map(target, |target| target.get_mut(field))
            }
            Value::Variant(target) => BorrowMut::try_map(vm_try!(target.borrow_mut()), |target| {
                match target.data_mut() {
                    VariantData::Struct(st) => st.get_mut(field),
                    _ => None,
                }
            }),
            _ => return VmResult::Ok(None),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return err(VmErrorKind::MissingField {
                    target: vm_try!(target.type_info()),
                    field: field.to_owned(),
                });
            }
        };

        VmResult::Ok(Some(value))
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_tuple_like_index_set(target: &Value, index: usize, value: Value) -> VmResult<bool> {
        match target {
            Value::Unit => VmResult::Ok(false),
            Value::Tuple(tuple) => {
                let mut tuple = vm_try!(tuple.borrow_mut());

                if let Some(target) = tuple.get_mut(index) {
                    *target = value;
                    return VmResult::Ok(true);
                }

                VmResult::Ok(false)
            }
            Value::Vec(vec) => {
                let mut vec = vm_try!(vec.borrow_mut());

                if let Some(target) = vec.get_mut(index) {
                    *target = value;
                    return VmResult::Ok(true);
                }

                VmResult::Ok(false)
            }
            Value::Result(result) => {
                let mut result = vm_try!(result.borrow_mut());

                let target = match &mut *result {
                    Result::Ok(ok) if index == 0 => ok,
                    Result::Err(err) if index == 1 => err,
                    _ => return VmResult::Ok(false),
                };

                *target = value;
                VmResult::Ok(true)
            }
            Value::Option(option) => {
                let mut option = vm_try!(option.borrow_mut());

                let target = match &mut *option {
                    Some(some) if index == 0 => some,
                    _ => return VmResult::Ok(false),
                };

                *target = value;
                VmResult::Ok(true)
            }
            Value::TupleStruct(tuple_struct) => {
                let mut tuple_struct = vm_try!(tuple_struct.borrow_mut());

                if let Some(target) = tuple_struct.get_mut(index) {
                    *target = value;
                    return VmResult::Ok(true);
                }

                VmResult::Ok(false)
            }
            Value::Variant(variant) => {
                let mut variant = vm_try!(variant.borrow_mut());

                if let VariantData::Tuple(data) = variant.data_mut() {
                    if let Some(target) = data.get_mut(index) {
                        *target = value;
                        return VmResult::Ok(true);
                    }
                }

                VmResult::Ok(false)
            }
            _ => VmResult::Ok(false),
        }
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_object_slot_index_get(
        &mut self,
        target: Value,
        string_slot: usize,
    ) -> VmResult<CallResult<Value>> {
        let index = vm_try!(self.unit.lookup_string(string_slot));

        match target {
            Value::Object(object) => {
                let object = vm_try!(object.borrow_ref());

                if let Some(value) = object.get(&***index) {
                    return VmResult::Ok(CallResult::Ok(value.clone()));
                }
            }
            Value::Struct(typed_object) => {
                let typed_object = vm_try!(typed_object.borrow_ref());

                if let Some(value) = typed_object.get(&***index) {
                    return VmResult::Ok(CallResult::Ok(value.clone()));
                }
            }
            Value::Variant(variant) => {
                let variant = vm_try!(variant.borrow_ref());

                if let VariantData::Struct(data) = variant.data() {
                    if let Some(value) = data.get(&***index) {
                        return VmResult::Ok(CallResult::Ok(value.clone()));
                    }
                }
            }
            target => {
                let hash = index.hash();

                return VmResult::Ok(
                    match vm_try!(self.call_field_fn(Protocol::GET, target, hash, ())) {
                        CallResult::Ok(()) => CallResult::Ok(vm_try!(self.stack.pop())),
                        CallResult::Unsupported(target) => CallResult::Unsupported(target),
                    },
                );
            }
        }

        err(VmErrorKind::ObjectIndexMissing { slot: string_slot })
    }

    fn try_object_slot_index_set(
        &mut self,
        target: Value,
        string_slot: usize,
        value: Value,
    ) -> VmResult<CallResult<()>> {
        let field = vm_try!(self.unit.lookup_string(string_slot));

        VmResult::Ok(match target {
            Value::Object(object) => {
                let mut object = vm_try!(object.borrow_mut());
                object.insert(field.as_str().to_owned(), value);
                return VmResult::Ok(CallResult::Ok(()));
            }
            Value::Struct(typed_object) => {
                let mut typed_object = vm_try!(typed_object.borrow_mut());

                if let Some(v) = typed_object.get_mut(field.as_str()) {
                    *v = value;
                    return VmResult::Ok(CallResult::Ok(()));
                }

                return err(VmErrorKind::MissingField {
                    field: field.as_str().to_owned(),
                    target: typed_object.type_info(),
                });
            }
            Value::Variant(variant) => {
                let mut variant = vm_try!(variant.borrow_mut());

                if let VariantData::Struct(data) = variant.data_mut() {
                    if let Some(v) = data.get_mut(field.as_str()) {
                        *v = value;
                        return VmResult::Ok(CallResult::Ok(()));
                    }
                }

                return err(VmErrorKind::MissingField {
                    field: field.as_str().to_owned(),
                    target: variant.type_info(),
                });
            }
            target => {
                let hash = field.hash();

                match vm_try!(self.call_field_fn(Protocol::SET, target, hash, (value,))) {
                    CallResult::Ok(()) => {
                        vm_try!(self.stack.pop());
                        CallResult::Ok(())
                    }
                    result => result,
                }
            }
        })
    }

    fn on_tuple<F, O>(&mut self, ty: TypeCheck, value: &Value, f: F) -> VmResult<Option<O>>
    where
        F: FnOnce(&[Value]) -> O,
    {
        VmResult::Ok(match (ty, value) {
            (TypeCheck::Tuple, Value::Tuple(tuple)) => Some(f(&vm_try!(tuple.borrow_ref()))),
            (TypeCheck::Vec, Value::Vec(vec)) => Some(f(&vm_try!(vec.borrow_ref()))),
            (TypeCheck::Result(v), Value::Result(result)) => {
                let result = vm_try!(result.borrow_ref());

                Some(match (v, &*result) {
                    (0, Result::Ok(ok)) => f(slice::from_ref(ok)),
                    (1, Result::Err(err)) => f(slice::from_ref(err)),
                    _ => return VmResult::Ok(None),
                })
            }
            (TypeCheck::Option(v), Value::Option(option)) => {
                let option = vm_try!(option.borrow_ref());

                Some(match (v, &*option) {
                    (0, Some(some)) => f(slice::from_ref(some)),
                    (1, None) => f(&[]),
                    _ => return VmResult::Ok(None),
                })
            }
            (TypeCheck::GeneratorState(v), Value::GeneratorState(state)) => {
                use crate::runtime::GeneratorState::*;
                let state = vm_try!(state.borrow_ref());

                Some(match (v, &*state) {
                    (0, Complete(complete)) => f(slice::from_ref(complete)),
                    (1, Yielded(yielded)) => f(slice::from_ref(yielded)),
                    _ => return VmResult::Ok(None),
                })
            }
            (TypeCheck::Unit, Value::Unit) => Some(f(&[])),
            _ => None,
        })
    }

    /// Internal implementation of the instance check.
    fn test_is_instance(&mut self, lhs: InstAddress, rhs: InstAddress) -> VmResult<bool> {
        let b = vm_try!(self.stack.address(rhs));
        let a = vm_try!(self.stack.address(lhs));

        let ty = match b {
            Value::Type(ty) => ty,
            _ => {
                return err(VmErrorKind::UnsupportedIs {
                    value: vm_try!(a.type_info()),
                    test_type: vm_try!(b.type_info()),
                });
            }
        };

        VmResult::Ok(vm_try!(a.type_hash()) == ty.into_hash())
    }

    fn internal_boolean_op(
        &mut self,
        bool_op: impl FnOnce(bool, bool) -> bool,
        op: &'static str,
        lhs: InstAddress,
        rhs: InstAddress,
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.address(rhs));
        let lhs = vm_try!(self.stack.address(lhs));

        let out = match (lhs, rhs) {
            (Value::Bool(lhs), Value::Bool(rhs)) => bool_op(lhs, rhs),
            (lhs, rhs) => {
                return err(VmErrorKind::UnsupportedBinaryOperation {
                    op,
                    lhs: vm_try!(lhs.type_info()),
                    rhs: vm_try!(rhs.type_info()),
                });
            }
        };

        self.stack.push(out);
        VmResult::Ok(())
    }

    /// Construct a future from calling an async function.
    fn call_generator_fn(&mut self, offset: usize, args: usize) -> Result<(), VmErrorKind> {
        let stack = self.stack.drain(args)?.collect::<Stack>();
        let mut vm = Self::with_stack(self.context.clone(), self.unit.clone(), stack);
        vm.ip = offset;
        self.stack.push(Generator::new(vm));
        Ok(())
    }

    /// Construct a stream from calling a function.
    fn call_stream_fn(&mut self, offset: usize, args: usize) -> Result<(), VmErrorKind> {
        let stack = self.stack.drain(args)?.collect::<Stack>();
        let mut vm = Self::with_stack(self.context.clone(), self.unit.clone(), stack);
        vm.ip = offset;
        self.stack.push(Stream::new(vm));
        Ok(())
    }

    /// Construct a future from calling a function.
    fn call_async_fn(&mut self, offset: usize, args: usize) -> Result<(), VmErrorKind> {
        let stack = self.stack.drain(args)?.collect::<Stack>();
        let mut vm = Self::with_stack(self.context.clone(), self.unit.clone(), stack);
        vm.ip = offset;
        let mut execution = vm.into_execution();
        self.stack
            .push(Future::new(async move { execution.async_complete().await }));
        Ok(())
    }

    /// Helper function to call the function at the given offset.
    fn call_offset_fn(
        &mut self,
        offset: usize,
        call: Call,
        args: usize,
    ) -> Result<bool, VmErrorKind> {
        let moved = match call {
            Call::Async => {
                self.call_async_fn(offset, args)?;
                false
            }
            Call::Immediate => {
                self.push_call_frame(offset, args)?;
                true
            }
            Call::Stream => {
                self.call_stream_fn(offset, args)?;
                false
            }
            Call::Generator => {
                self.call_generator_fn(offset, args)?;
                false
            }
        };

        Ok(moved)
    }

    fn internal_num_assign(
        &mut self,
        target: InstTarget,
        protocol: Protocol,
        error: fn() -> VmErrorKind,
        integer_op: fn(i64, i64) -> Option<i64>,
        float_op: fn(f64, f64) -> f64,
    ) -> VmResult<()> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs) {
            TargetValue::Value(lhs, rhs) => match (lhs, rhs) {
                (Value::Integer(lhs), Value::Integer(rhs)) => {
                    let out = vm_try!(integer_op(*lhs, rhs).ok_or_else(error));
                    *lhs = out;
                    return VmResult::Ok(());
                }
                (Value::Float(lhs), Value::Float(rhs)) => {
                    let out = float_op(*lhs, rhs);
                    *lhs = out;
                    return VmResult::Ok(());
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
    ) -> VmResult<()> {
        match fallback {
            TargetFallback::Value(lhs, rhs) => {
                match vm_try!(self.call_instance_fn(lhs, protocol, (&rhs,))) {
                    CallResult::Ok(()) => vm_try!(<()>::from_value(vm_try!(self.stack.pop()))),
                    CallResult::Unsupported(lhs) => {
                        return err(VmErrorKind::UnsupportedBinaryOperation {
                            op: protocol.name,
                            lhs: vm_try!(lhs.type_info()),
                            rhs: vm_try!(rhs.type_info()),
                        });
                    }
                };

                VmResult::Ok(())
            }
            TargetFallback::Field(lhs, hash, rhs) => {
                if let CallResult::Unsupported(lhs) =
                    vm_try!(self.call_field_fn(protocol, lhs.clone(), hash, (rhs,)))
                {
                    return err(VmErrorKind::UnsupportedObjectSlotIndexGet {
                        target: vm_try!(lhs.type_info()),
                    });
                }

                let value = vm_try!(self.stack.pop());
                vm_try!(<()>::from_value(value));
                VmResult::Ok(())
            }
            TargetFallback::Index(lhs, ..) => err(VmErrorKind::UnsupportedTupleIndexGet {
                target: vm_try!(lhs.type_info()),
            }),
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
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.address(rhs));
        let lhs = vm_try!(self.stack.address(lhs));

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                self.stack
                    .push(vm_try!(integer_op(lhs, rhs).ok_or_else(error)));
                return VmResult::Ok(());
            }
            (Value::Float(lhs), Value::Float(rhs)) => {
                self.stack.push(float_op(lhs, rhs));
                return VmResult::Ok(());
            }
            (lhs, rhs) => (lhs, rhs),
        };

        if let CallResult::Unsupported(lhs) = vm_try!(self.call_instance_fn(lhs, protocol, (&rhs,)))
        {
            err(VmErrorKind::UnsupportedBinaryOperation {
                op: protocol.name,
                lhs: vm_try!(lhs.type_info()),
                rhs: vm_try!(rhs.type_info()),
            })
        } else {
            VmResult::Ok(())
        }
    }

    /// Internal impl of a numeric operation.
    fn internal_infallible_bitwise(
        &mut self,
        protocol: Protocol,
        integer_op: fn(i64, i64) -> i64,
        lhs: InstAddress,
        rhs: InstAddress,
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.address(rhs));
        let lhs = vm_try!(self.stack.address(lhs));

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                self.stack.push(integer_op(lhs, rhs));
                return VmResult::Ok(());
            }
            (lhs, rhs) => (lhs, rhs),
        };

        if let CallResult::Unsupported(lhs) = vm_try!(self.call_instance_fn(lhs, protocol, (&rhs,)))
        {
            err(VmErrorKind::UnsupportedBinaryOperation {
                op: protocol.name,
                lhs: vm_try!(lhs.type_info()),
                rhs: vm_try!(rhs.type_info()),
            })
        } else {
            VmResult::Ok(())
        }
    }

    /// Internal impl of a numeric operation.
    fn internal_infallible_bitwise_bool(
        &mut self,
        protocol: Protocol,
        integer_op: fn(i64, i64) -> i64,
        bool_op: fn(bool, bool) -> bool,
        lhs: InstAddress,
        rhs: InstAddress,
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.address(rhs));
        let lhs = vm_try!(self.stack.address(lhs));

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                self.stack.push(integer_op(lhs, rhs));
                return VmResult::Ok(());
            }
            (Value::Bool(lhs), Value::Bool(rhs)) => {
                self.stack.push(bool_op(lhs, rhs));
                return VmResult::Ok(());
            }
            (lhs, rhs) => (lhs, rhs),
        };

        if let CallResult::Unsupported(lhs) = vm_try!(self.call_instance_fn(lhs, protocol, (&rhs,)))
        {
            err(VmErrorKind::UnsupportedBinaryOperation {
                op: protocol.name,
                lhs: vm_try!(lhs.type_info()),
                rhs: vm_try!(rhs.type_info()),
            })
        } else {
            VmResult::Ok(())
        }
    }

    fn internal_infallible_bitwise_assign(
        &mut self,
        target: InstTarget,
        protocol: Protocol,
        integer_op: fn(&mut i64, i64),
    ) -> VmResult<()> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs) {
            TargetValue::Value(lhs, rhs) => match (lhs, rhs) {
                (Value::Integer(lhs), Value::Integer(rhs)) => {
                    integer_op(lhs, rhs);
                    return VmResult::Ok(());
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
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.address(rhs));
        let lhs = vm_try!(self.stack.address(lhs));

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                self.stack
                    .push(vm_try!(integer_op(lhs, rhs).ok_or_else(error)));
                return VmResult::Ok(());
            }
            (lhs, rhs) => (lhs, rhs),
        };

        if let CallResult::Unsupported(lhs) = vm_try!(self.call_instance_fn(lhs, protocol, (&rhs,)))
        {
            err(VmErrorKind::UnsupportedBinaryOperation {
                op: protocol.name,
                lhs: vm_try!(lhs.type_info()),
                rhs: vm_try!(rhs.type_info()),
            })
        } else {
            VmResult::Ok(())
        }
    }

    fn internal_bitwise_assign(
        &mut self,
        target: InstTarget,
        protocol: Protocol,
        error: fn() -> VmErrorKind,
        integer_op: fn(i64, i64) -> Option<i64>,
    ) -> VmResult<()> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs) {
            TargetValue::Value(lhs, rhs) => match (lhs, rhs) {
                (Value::Integer(lhs), Value::Integer(rhs)) => {
                    let out = vm_try!(integer_op(*lhs, rhs).ok_or_else(error));
                    *lhs = out;
                    return VmResult::Ok(());
                }
                (lhs, rhs) => TargetFallback::Value(lhs.clone(), rhs),
            },
            TargetValue::Fallback(fallback) => fallback,
        };

        self.target_fallback(fallback, protocol)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_await(&mut self) -> VmResult<Shared<Future>> {
        let value = vm_try!(self.stack.pop());
        value.into_shared_future()
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_select(&mut self, len: usize) -> VmResult<Option<Select>> {
        let futures = futures_util::stream::FuturesUnordered::new();

        for (branch, value) in vm_try!(self.stack.drain(len)).enumerate() {
            let future = vm_try!(vm_try!(value.into_shared_future()).into_mut());

            if !future.is_completed() {
                futures.push(SelectFuture::new(branch, future));
            }
        }

        // NB: nothing to poll.
        if futures.is_empty() {
            self.stack.push(());
            return VmResult::Ok(None);
        }

        VmResult::Ok(Some(Select::new(futures)))
    }

    /// Pop a number of values from the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_popn(&mut self, n: usize) -> VmResult<()> {
        vm_try!(self.stack.popn(n));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_push(&mut self, value: InstValue) -> VmResult<()> {
        self.stack.push(value.into_value());
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_pop(&mut self) -> VmResult<()> {
        vm_try!(self.stack.pop());
        VmResult::Ok(())
    }

    /// pop-and-jump-if-not instruction.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_pop_and_jump_if_not(&mut self, count: usize, jump: usize) -> VmResult<()> {
        if vm_try!(vm_try!(self.stack.pop()).into_bool()) {
            return VmResult::Ok(());
        }

        vm_try!(self.stack.popn(count));
        self.ip = vm_try!(self.unit.translate(jump));
        VmResult::Ok(())
    }

    /// Pop a number of values from the stack, while preserving the top of the
    /// stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_clean(&mut self, n: usize) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());
        vm_try!(self.op_popn(n));
        self.stack.push(value);
        VmResult::Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_copy(&mut self, offset: usize) -> VmResult<()> {
        let value = vm_try!(self.stack.at_offset(offset)).clone();
        self.stack.push(value);
        VmResult::Ok(())
    }

    /// Move a value from a position relative to the top of the stack, to the
    /// top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_move(&mut self, offset: usize) -> VmResult<()> {
        let value = vm_try!(self.stack.at_offset(offset)).clone();
        self.stack.push(vm_try!(value.take()));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_drop(&mut self, offset: usize) -> VmResult<()> {
        let _ = vm_try!(self.stack.at_offset(offset));
        VmResult::Ok(())
    }

    /// Duplicate the value at the top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_dup(&mut self) -> VmResult<()> {
        let value = vm_try!(self.stack.last()).clone();
        self.stack.push(value);
        VmResult::Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_replace(&mut self, offset: usize) -> VmResult<()> {
        let mut value = vm_try!(self.stack.pop());
        let stack_value = vm_try!(self.stack.at_offset_mut(offset));
        mem::swap(stack_value, &mut value);
        VmResult::Ok(())
    }

    /// Perform a jump operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_jump(&mut self, jump: usize) -> VmResult<()> {
        self.ip = vm_try!(self.unit.translate(jump));
        VmResult::Ok(())
    }

    /// Perform a conditional jump operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_jump_if(&mut self, jump: usize) -> VmResult<()> {
        if vm_try!(vm_try!(self.stack.pop()).into_bool()) {
            self.ip = vm_try!(self.unit.translate(jump));
        }

        VmResult::Ok(())
    }

    /// Perform a conditional jump operation. Pops the stack if the jump is
    /// not performed.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_jump_if_or_pop(&mut self, jump: usize) -> VmResult<()> {
        if vm_try!(vm_try!(self.stack.last()).as_bool()) {
            self.ip = vm_try!(self.unit.translate(jump));
        } else {
            vm_try!(self.stack.pop());
        }

        VmResult::Ok(())
    }

    /// Perform a conditional jump operation. Pops the stack if the jump is
    /// not performed.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_jump_if_not_or_pop(&mut self, jump: usize) -> VmResult<()> {
        if !vm_try!(vm_try!(self.stack.last()).as_bool()) {
            self.ip = vm_try!(self.unit.translate(jump));
        } else {
            vm_try!(self.stack.pop());
        }

        VmResult::Ok(())
    }

    /// Perform a branch-conditional jump operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_jump_if_branch(&mut self, branch: i64, jump: usize) -> VmResult<()> {
        if let Some(Value::Integer(current)) = self.stack.peek() {
            if *current == branch {
                self.ip = vm_try!(self.unit.translate(jump));
                vm_try!(self.stack.pop());
            }
        }

        VmResult::Ok(())
    }

    /// Construct a new vec.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_vec(&mut self, count: usize) -> VmResult<()> {
        let vec = Vec::from(vm_try!(self.stack.pop_sequence(count)));
        self.stack.push(Shared::new(vec));
        VmResult::Ok(())
    }

    /// Construct a new tuple.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple(&mut self, count: usize) -> VmResult<()> {
        let tuple = vm_try!(self.stack.pop_sequence(count));
        self.stack.push(Tuple::from(tuple));
        VmResult::Ok(())
    }

    /// Construct a new tuple with a fixed number of arguments.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_n(&mut self, args: &[InstAddress]) -> VmResult<()> {
        let mut tuple = vec![Value::Unit; args.len()];

        for (n, arg) in args.iter().enumerate().rev() {
            tuple[n] = vm_try!(self.stack.address(*arg));
        }

        self.stack.push(Tuple::from(tuple));
        VmResult::Ok(())
    }

    /// Push the tuple that is on top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_push_tuple(&mut self) -> VmResult<()> {
        let tuple = vm_try!(vm_try!(self.stack.pop()).into_tuple());
        self.stack
            .extend(vm_try!(tuple.borrow_ref()).iter().cloned());
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_not(&mut self) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        let value = match value {
            Value::Bool(value) => Value::from(!value),
            Value::Integer(value) => Value::from(!value),
            other => {
                let operand = vm_try!(other.type_info());
                return err(VmErrorKind::UnsupportedUnaryOperation { op: "!", operand });
            }
        };

        self.stack.push(value);
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_neg(&mut self) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        let value = match value {
            Value::Float(value) => Value::from(-value),
            Value::Integer(value) => Value::from(-value),
            other => {
                let operand = vm_try!(other.type_info());
                return err(VmErrorKind::UnsupportedUnaryOperation { op: "-", operand });
            }
        };

        self.stack.push(value);
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_op(&mut self, op: InstOp, lhs: InstAddress, rhs: InstAddress) -> VmResult<()> {
        match op {
            InstOp::Add => {
                vm_try!(self.internal_num(
                    Protocol::ADD,
                    || VmErrorKind::Overflow,
                    i64::checked_add,
                    ops::Add::add,
                    lhs,
                    rhs,
                ));
            }
            InstOp::Sub => {
                vm_try!(self.internal_num(
                    Protocol::SUB,
                    || VmErrorKind::Underflow,
                    i64::checked_sub,
                    ops::Sub::sub,
                    lhs,
                    rhs,
                ));
            }
            InstOp::Mul => {
                vm_try!(self.internal_num(
                    Protocol::MUL,
                    || VmErrorKind::Overflow,
                    i64::checked_mul,
                    ops::Mul::mul,
                    lhs,
                    rhs,
                ));
            }
            InstOp::Div => {
                vm_try!(self.internal_num(
                    Protocol::DIV,
                    || VmErrorKind::DivideByZero,
                    i64::checked_div,
                    ops::Div::div,
                    lhs,
                    rhs,
                ));
            }
            InstOp::Rem => {
                vm_try!(self.internal_num(
                    Protocol::REM,
                    || VmErrorKind::DivideByZero,
                    i64::checked_rem,
                    ops::Rem::rem,
                    lhs,
                    rhs,
                ));
            }
            InstOp::BitAnd => {
                use ops::BitAnd as _;
                vm_try!(self.internal_infallible_bitwise_bool(
                    Protocol::BIT_AND,
                    i64::bitand,
                    bool::bitand,
                    lhs,
                    rhs,
                ));
            }
            InstOp::BitXor => {
                use ops::BitXor as _;
                vm_try!(self.internal_infallible_bitwise_bool(
                    Protocol::BIT_XOR,
                    i64::bitxor,
                    bool::bitxor,
                    lhs,
                    rhs,
                ));
            }
            InstOp::BitOr => {
                use ops::BitOr as _;
                vm_try!(self.internal_infallible_bitwise_bool(
                    Protocol::BIT_OR,
                    i64::bitor,
                    bool::bitor,
                    lhs,
                    rhs,
                ));
            }
            InstOp::Shl => {
                vm_try!(self.internal_bitwise(
                    Protocol::SHL,
                    || VmErrorKind::Overflow,
                    |a, b| a.checked_shl(u32::try_from(b).ok()?),
                    lhs,
                    rhs,
                ));
            }
            InstOp::Shr => {
                vm_try!(self.internal_infallible_bitwise(Protocol::SHR, ops::Shr::shr, lhs, rhs));
            }
            InstOp::Gt => {
                vm_try!(self.internal_boolean_ops(|a, b| a > b, |a, b| a > b, ">", lhs, rhs));
            }
            InstOp::Gte => {
                vm_try!(self.internal_boolean_ops(|a, b| a >= b, |a, b| a >= b, ">=", lhs, rhs));
            }
            InstOp::Lt => {
                vm_try!(self.internal_boolean_ops(|a, b| a < b, |a, b| a < b, "<", lhs, rhs));
            }
            InstOp::Lte => {
                vm_try!(self.internal_boolean_ops(|a, b| a <= b, |a, b| a <= b, "<=", lhs, rhs));
            }
            InstOp::Eq => {
                let rhs = vm_try!(self.stack.address(rhs));
                let lhs = vm_try!(self.stack.address(lhs));
                let test = vm_try!(Value::value_ptr_eq(self, &lhs, &rhs));
                self.stack.push(test);
            }
            InstOp::Neq => {
                let rhs = vm_try!(self.stack.address(rhs));
                let lhs = vm_try!(self.stack.address(lhs));
                let test = vm_try!(Value::value_ptr_eq(self, &lhs, &rhs));
                self.stack.push(!test);
            }
            InstOp::And => {
                vm_try!(self.internal_boolean_op(|a, b| a && b, "&&", lhs, rhs));
            }
            InstOp::Or => {
                vm_try!(self.internal_boolean_op(|a, b| a || b, "||", lhs, rhs));
            }
            InstOp::Is => {
                let is_instance = vm_try!(self.test_is_instance(lhs, rhs));
                self.stack.push(is_instance);
            }
            InstOp::IsNot => {
                let is_instance = vm_try!(self.test_is_instance(lhs, rhs));
                self.stack.push(!is_instance);
            }
        }

        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_assign(&mut self, target: InstTarget, op: InstAssignOp) -> VmResult<()> {
        match op {
            InstAssignOp::Add => {
                vm_try!(self.internal_num_assign(
                    target,
                    Protocol::ADD_ASSIGN,
                    || VmErrorKind::Overflow,
                    i64::checked_add,
                    ops::Add::add,
                ));
            }
            InstAssignOp::Sub => {
                vm_try!(self.internal_num_assign(
                    target,
                    Protocol::SUB_ASSIGN,
                    || VmErrorKind::Underflow,
                    i64::checked_sub,
                    ops::Sub::sub,
                ));
            }
            InstAssignOp::Mul => {
                vm_try!(self.internal_num_assign(
                    target,
                    Protocol::MUL_ASSIGN,
                    || VmErrorKind::Overflow,
                    i64::checked_mul,
                    ops::Mul::mul,
                ));
            }
            InstAssignOp::Div => {
                vm_try!(self.internal_num_assign(
                    target,
                    Protocol::DIV_ASSIGN,
                    || VmErrorKind::DivideByZero,
                    i64::checked_div,
                    ops::Div::div,
                ));
            }
            InstAssignOp::Rem => {
                vm_try!(self.internal_num_assign(
                    target,
                    Protocol::REM_ASSIGN,
                    || VmErrorKind::DivideByZero,
                    i64::checked_rem,
                    ops::Rem::rem,
                ));
            }
            InstAssignOp::BitAnd => {
                vm_try!(self.internal_infallible_bitwise_assign(
                    target,
                    Protocol::BIT_AND_ASSIGN,
                    ops::BitAndAssign::bitand_assign,
                ));
            }
            InstAssignOp::BitXor => {
                vm_try!(self.internal_infallible_bitwise_assign(
                    target,
                    Protocol::BIT_XOR_ASSIGN,
                    ops::BitXorAssign::bitxor_assign,
                ));
            }
            InstAssignOp::BitOr => {
                vm_try!(self.internal_infallible_bitwise_assign(
                    target,
                    Protocol::BIT_OR_ASSIGN,
                    ops::BitOrAssign::bitor_assign,
                ));
            }
            InstAssignOp::Shl => {
                vm_try!(self.internal_bitwise_assign(
                    target,
                    Protocol::SHL_ASSIGN,
                    || VmErrorKind::Overflow,
                    |a, b| a.checked_shl(u32::try_from(b).ok()?),
                ));
            }
            InstAssignOp::Shr => {
                vm_try!(self.internal_infallible_bitwise_assign(
                    target,
                    Protocol::SHR_ASSIGN,
                    ops::ShrAssign::shr_assign,
                ));
            }
        }

        VmResult::Ok(())
    }

    /// Perform an index set operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_index_set(&mut self) -> VmResult<()> {
        let index = vm_try!(self.stack.pop());
        let target = vm_try!(self.stack.pop());
        let value = vm_try!(self.stack.pop());

        // This is a useful pattern.
        #[allow(clippy::never_loop)]
        loop {
            // NB: local storage for string.
            let local_field;

            let field = match &index {
                Value::String(string) => {
                    local_field = vm_try!(string.borrow_ref());
                    local_field.as_str()
                }
                Value::StaticString(string) => string.as_ref(),
                _ => break,
            };

            match &target {
                Value::Object(object) => {
                    let mut object = vm_try!(object.borrow_mut());
                    object.insert(field.to_owned(), value);
                    return VmResult::Ok(());
                }
                Value::Struct(typed_object) => {
                    let mut typed_object = vm_try!(typed_object.borrow_mut());

                    if let Some(v) = typed_object.get_mut(field) {
                        *v = value;
                        return VmResult::Ok(());
                    }

                    return err(VmErrorKind::MissingField {
                        field: field.to_owned(),
                        target: typed_object.type_info(),
                    });
                }
                Value::Variant(variant) => {
                    let mut variant = vm_try!(variant.borrow_mut());

                    if let VariantData::Struct(st) = variant.data_mut() {
                        if let Some(v) = st.get_mut(field) {
                            *v = value;
                            return VmResult::Ok(());
                        }
                    }

                    return err(VmErrorKind::MissingField {
                        field: field.to_owned(),
                        target: variant.type_info(),
                    });
                }
                _ => {
                    break;
                }
            }
        }

        if let CallResult::Unsupported(target) =
            vm_try!(self.call_instance_fn(target, Protocol::INDEX_SET, (&index, &value)))
        {
            err(VmErrorKind::UnsupportedIndexSet {
                target: vm_try!(target.type_info()),
                index: vm_try!(index.type_info()),
                value: vm_try!(value.type_info()),
            })
        } else {
            // Calling index set should not produce a value on the stack, but all
            // handler functions to produce a value. So pop it here.
            vm_try!(<()>::from_value(vm_try!(self.stack.pop())));
            VmResult::Ok(())
        }
    }

    #[inline]
    fn op_return_internal(
        &mut self,
        return_value: Value,
        clean: usize,
    ) -> Result<bool, VmErrorKind> {
        if clean > 0 {
            self.stack.popn(clean)?;
        }

        let exit = self.pop_call_frame()?;
        self.stack.push(return_value);
        Ok(exit)
    }

    fn lookup_function_by_hash(&self, hash: Hash) -> Result<Function, VmErrorKind> {
        Ok(match self.unit.function(hash) {
            Some(info) => match info {
                UnitFn::Offset { offset, call, args } => Function::from_vm_offset(
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
                    .function(hash)
                    .ok_or(VmErrorKind::MissingContextFunction { hash })?;

                Function::from_handler(handler.clone(), hash)
            }
        })
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_return(&mut self, address: InstAddress, clean: usize) -> Result<bool, VmErrorKind> {
        let return_value = self.stack.address(address)?;
        self.op_return_internal(return_value, clean)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_return_unit(&mut self) -> Result<bool, VmErrorKind> {
        let exit = self.pop_call_frame()?;
        self.stack.push(());
        Ok(exit)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_load_instance_fn(&mut self, hash: Hash) -> Result<(), VmError> {
        let instance = self.stack.pop()?;
        let ty = instance.type_hash()?;
        let hash = Hash::associated_function(ty, hash);
        self.stack.push(Value::Type(Type::new(hash)));
        Ok(())
    }

    /// Perform an index get operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_index_get(&mut self, target: InstAddress, index: InstAddress) -> VmResult<()> {
        let index = vm_try!(self.stack.address(index));
        let target = vm_try!(self.stack.address_ref(target));

        match &index {
            Value::String(string) => {
                let string_ref = vm_try!(string.borrow_ref());

                if let Some(value) = vm_try!(Self::try_object_like_index_get(
                    &target,
                    string_ref.as_str()
                )) {
                    self.stack.push(value);
                    return VmResult::Ok(());
                }
            }
            Value::StaticString(string) => {
                if let Some(value) =
                    vm_try!(Self::try_object_like_index_get(&target, string.as_ref()))
                {
                    self.stack.push(value);
                    return VmResult::Ok(());
                }
            }
            Value::Integer(index) => {
                let index = match (*index).try_into() {
                    Result::Ok(index) => index,
                    Result::Err(..) => {
                        return err(VmErrorKind::MissingIndex {
                            target: vm_try!(target.type_info()),
                            index: VmIntegerRepr::from(*index),
                        });
                    }
                };

                if let Some(value) = vm_try!(Self::try_tuple_like_index_get(&target, index)) {
                    self.stack.push(value);
                    return VmResult::Ok(());
                }
            }
            _ => (),
        }

        let target = target.into_owned();

        if let CallResult::Unsupported(target) =
            vm_try!(self.call_instance_fn(target, Protocol::INDEX_GET, (&index,)))
        {
            err(VmErrorKind::UnsupportedIndexGet {
                target: vm_try!(target.type_info()),
                index: vm_try!(index.type_info()),
            })
        } else {
            VmResult::Ok(())
        }
    }

    /// Perform an index get operation specialized for tuples.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_index_get(&mut self, index: usize) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        if let Some(value) = vm_try!(Self::try_tuple_like_index_get(&value, index)) {
            self.stack.push(value);
            return VmResult::Ok(());
        }

        if let CallResult::Unsupported(value) =
            vm_try!(self.call_index_fn(Protocol::GET, value, index, ()))
        {
            return err(VmErrorKind::UnsupportedTupleIndexGet {
                target: vm_try!(value.type_info()),
            });
        }

        // NB: should leave a value on the stack.
        VmResult::Ok(())
    }

    /// Perform an index get operation specialized for tuples.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_index_set(&mut self, index: usize) -> VmResult<()> {
        let tuple = vm_try!(self.stack.pop());
        let value = vm_try!(self.stack.pop());

        if vm_try!(Self::try_tuple_like_index_set(&tuple, index, value)) {
            return VmResult::Ok(());
        }

        err(VmErrorKind::UnsupportedTupleIndexSet {
            target: vm_try!(tuple.type_info()),
        })
    }

    /// Perform an index get operation specialized for tuples.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_index_get_at(&mut self, offset: usize, index: usize) -> VmResult<()> {
        let value = vm_try!(self.stack.at_offset(offset));

        if let Some(value) = vm_try!(Self::try_tuple_like_index_get(value, index)) {
            self.stack.push(value);
            return VmResult::Ok(());
        }

        let value = value.clone();

        if let CallResult::Unsupported(value) =
            vm_try!(self.call_index_fn(Protocol::GET, value, index, ()))
        {
            return err(VmErrorKind::UnsupportedTupleIndexGet {
                target: vm_try!(value.type_info()),
            });
        }

        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_bool(&mut self, boolean: bool) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        self.stack.push(match value {
            Value::Bool(actual) => actual == boolean,
            _ => false,
        });

        VmResult::Ok(())
    }

    /// Perform a specialized index get operation on an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object_index_get(&mut self, string_slot: usize) -> VmResult<()> {
        let target = vm_try!(self.stack.pop());

        match vm_try!(self.try_object_slot_index_get(target, string_slot)) {
            CallResult::Ok(value) => {
                self.stack.push(value);
                VmResult::Ok(())
            }
            CallResult::Unsupported(target) => err(VmErrorKind::UnsupportedObjectSlotIndexGet {
                target: vm_try!(target.type_info()),
            }),
        }
    }

    /// Perform a specialized index set operation on an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object_index_set(&mut self, string_slot: usize) -> VmResult<()> {
        let target = vm_try!(self.stack.pop());
        let value = vm_try!(self.stack.pop());

        if let CallResult::Unsupported(target) =
            vm_try!(self.try_object_slot_index_set(target, string_slot, value))
        {
            err(VmErrorKind::UnsupportedObjectSlotIndexSet {
                target: vm_try!(target.type_info()),
            })
        } else {
            VmResult::Ok(())
        }
    }

    /// Perform a specialized index get operation on an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object_index_get_at(&mut self, offset: usize, string_slot: usize) -> VmResult<()> {
        let target = vm_try!(self.stack.at_offset(offset)).clone();

        match vm_try!(self.try_object_slot_index_get(target, string_slot)) {
            CallResult::Ok(value) => {
                self.stack.push(value);
                VmResult::Ok(())
            }
            CallResult::Unsupported(target) => err(VmErrorKind::UnsupportedObjectSlotIndexGet {
                target: vm_try!(target.type_info()),
            }),
        }
    }

    /// Operation to allocate an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object(&mut self, slot: usize) -> VmResult<()> {
        let keys = vm_try!(self
            .unit
            .lookup_object_keys(slot)
            .ok_or(VmErrorKind::MissingStaticObjectKeys { slot }));

        let mut object = Object::with_capacity(keys.len());
        let values = vm_try!(self.stack.drain(keys.len()));

        for (key, value) in keys.iter().zip(values) {
            object.insert(key.clone(), value);
        }

        self.stack.push(Shared::new(object));
        VmResult::Ok(())
    }

    /// Operation to allocate an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_range(&mut self, limits: InstRangeLimits) -> VmResult<()> {
        let end = vm_try!(Option::<Value>::from_value(vm_try!(self.stack.pop())));
        let start = vm_try!(Option::<Value>::from_value(vm_try!(self.stack.pop())));

        let limits = match limits {
            InstRangeLimits::HalfOpen => RangeLimits::HalfOpen,
            InstRangeLimits::Closed => RangeLimits::Closed,
        };

        let range = Range::new(start, end, limits);
        self.stack.push(Shared::new(range));
        VmResult::Ok(())
    }

    /// Operation to allocate an empty struct.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_empty_struct(&mut self, hash: Hash) -> VmResult<()> {
        let rtti = vm_try!(self
            .unit
            .lookup_rtti(hash)
            .ok_or(VmErrorKind::MissingRtti { hash }));

        self.stack.push(UnitStruct { rtti: rtti.clone() });
        VmResult::Ok(())
    }

    /// Operation to allocate an object struct.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_struct(&mut self, hash: Hash, slot: usize) -> VmResult<()> {
        let keys = vm_try!(self
            .unit
            .lookup_object_keys(slot)
            .ok_or(VmErrorKind::MissingStaticObjectKeys { slot }));

        let rtti = vm_try!(self
            .unit
            .lookup_rtti(hash)
            .ok_or(VmErrorKind::MissingRtti { hash }));

        let values = vm_try!(self.stack.drain(keys.len()));
        let mut data = Object::with_capacity(keys.len());

        for (key, value) in keys.iter().zip(values) {
            data.insert(key.clone(), value);
        }

        self.stack.push(Struct {
            rtti: rtti.clone(),
            data,
        });

        VmResult::Ok(())
    }

    /// Operation to allocate an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_unit_variant(&mut self, hash: Hash) -> VmResult<()> {
        let rtti = vm_try!(self
            .unit
            .lookup_variant_rtti(hash)
            .ok_or(VmErrorKind::MissingVariantRtti { hash }));

        self.stack.push(Variant::unit(rtti.clone()));
        VmResult::Ok(())
    }

    /// Operation to allocate an object variant.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object_variant(&mut self, hash: Hash, slot: usize) -> VmResult<()> {
        let keys = vm_try!(self
            .unit
            .lookup_object_keys(slot)
            .ok_or(VmErrorKind::MissingStaticObjectKeys { slot }));

        let rtti = vm_try!(self
            .unit
            .lookup_variant_rtti(hash)
            .ok_or(VmErrorKind::MissingVariantRtti { hash }));

        let mut data = Object::with_capacity(keys.len());
        let values = vm_try!(self.stack.drain(keys.len()));

        for (key, value) in keys.iter().zip(values) {
            data.insert(key.clone(), value);
        }

        self.stack.push(Variant::struct_(rtti.clone(), data));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_string(&mut self, slot: usize) -> VmResult<()> {
        let string = vm_try!(self.unit.lookup_string(slot));
        self.stack.push(string.clone());
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_bytes(&mut self, slot: usize) -> VmResult<()> {
        let bytes = vm_try!(self.unit.lookup_bytes(slot)).to_owned();
        self.stack.push(Bytes::from_vec(bytes));
        VmResult::Ok(())
    }

    /// Optimize operation to perform string concatenation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_string_concat(&mut self, len: usize, size_hint: usize) -> VmResult<()> {
        let values = vm_try!(self.stack.drain(len)).collect::<vec::Vec<_>>();

        let mut out = String::with_capacity(size_hint);
        let mut buf = String::with_capacity(16);

        for value in values {
            buf.clear();

            if let Result::Err(fmt::Error) =
                vm_try!(value.string_display_with(&mut out, &mut buf, &mut *self))
            {
                return err(VmErrorKind::FormatError);
            }
        }

        self.stack.push(out);
        VmResult::Ok(())
    }

    /// Push a format specification onto the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_format(&mut self, spec: FormatSpec) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());
        self.stack.push(Format { value, spec });
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_is_unit(&mut self) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());
        self.stack.push(matches!(value, Value::Unit));
        VmResult::Ok(())
    }

    /// Perform the try operation on the given stack location.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_try(&mut self, address: InstAddress, clean: usize, preserve: bool) -> VmResult<bool> {
        let return_value = vm_try!(self.stack.address(address));

        let unwrapped_value = match &return_value {
            Value::Result(result) => match &*vm_try!(result.borrow_ref()) {
                Result::Ok(value) => Some(value.clone()),
                Result::Err(..) => None,
            },
            Value::Option(option) => (*vm_try!(option.borrow_ref())).clone(),
            other => {
                return err(VmErrorKind::UnsupportedTryOperand {
                    actual: vm_try!(other.type_info()),
                });
            }
        };

        if let Some(value) = unwrapped_value {
            if preserve {
                self.stack.push(value);
            }

            VmResult::Ok(false)
        } else {
            VmResult::Ok(vm_try!(self.op_return_internal(return_value, clean)))
        }
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_byte(&mut self, byte: u8) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        self.stack.push(match value {
            Value::Byte(actual) => actual == byte,
            _ => false,
        });

        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_character(&mut self, character: char) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        self.stack.push(match value {
            Value::Char(actual) => actual == character,
            _ => false,
        });

        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_integer(&mut self, integer: i64) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        self.stack.push(match value {
            Value::Integer(actual) => actual == integer,
            _ => false,
        });

        VmResult::Ok(())
    }

    /// Test if the top of stack is equal to the string at the given static
    /// string slot.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_string(&mut self, slot: usize) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        let equal = match value {
            Value::String(actual) => {
                let string = vm_try!(self.unit.lookup_string(slot));
                let actual = vm_try!(actual.borrow_ref());
                *actual == ***string
            }
            Value::StaticString(actual) => {
                let string = vm_try!(self.unit.lookup_string(slot));
                **actual == ***string
            }
            _ => false,
        };

        self.stack.push(Value::Bool(equal));
        VmResult::Ok(())
    }

    /// Test if the top of stack is equal to the string at the given static
    /// bytes slot.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_bytes(&mut self, slot: usize) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        let equal = match value {
            Value::Bytes(actual) => {
                let bytes = vm_try!(self.unit.lookup_bytes(slot));
                let actual = vm_try!(actual.borrow_ref());
                *actual == *bytes
            }
            _ => false,
        };

        self.stack.push(Value::Bool(equal));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_match_sequence(&mut self, ty: TypeCheck, len: usize, exact: bool) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        let result = vm_try!(self.on_tuple(ty, &value, move |tuple| {
            if exact {
                tuple.len() == len
            } else {
                tuple.len() >= len
            }
        }));

        self.stack.push(Value::Bool(result.unwrap_or_default()));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_match_type(&mut self, hash: Hash) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());
        let is_match = vm_try!(value.type_hash()) == hash;
        self.stack.push(is_match);
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_match_variant(
        &mut self,
        enum_hash: Hash,
        variant_hash: Hash,
        index: usize,
    ) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        let is_match = match &value {
            Value::Variant(variant) => vm_try!(variant.borrow_ref()).rtti().hash == variant_hash,
            Value::Any(any) => {
                let hash = vm_try!(any.borrow_ref()).type_hash();

                if hash == enum_hash {
                    match vm_try!(self.call_instance_fn(value, Protocol::IS_VARIANT, (index,))) {
                        CallResult::Ok(()) => vm_try!(vm_try!(self.stack.pop()).as_bool()),
                        CallResult::Unsupported(..) => false,
                    }
                } else {
                    false
                }
            }
            _ => false,
        };

        self.stack.push(is_match);
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_match_builtin(&mut self, type_check: TypeCheck) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        let is_match = match (type_check, value) {
            (TypeCheck::Tuple, Value::Tuple(..)) => true,
            (TypeCheck::Vec, Value::Vec(..)) => true,
            (TypeCheck::Result(v), Value::Result(result)) => {
                let result = vm_try!(result.borrow_ref());

                match (v, &*result) {
                    (0, Result::Ok(..)) => true,
                    (1, Result::Err(..)) => true,
                    _ => false,
                }
            }
            (TypeCheck::Option(v), Value::Option(option)) => {
                let option = vm_try!(option.borrow_ref());

                match (v, &*option) {
                    (0, Some(..)) => true,
                    (1, None) => true,
                    _ => false,
                }
            }
            (TypeCheck::GeneratorState(v), Value::GeneratorState(state)) => {
                use crate::runtime::GeneratorState::*;
                let state = vm_try!(state.borrow_ref());

                match (v, &*state) {
                    (0, Complete(..)) => true,
                    (1, Yielded(..)) => true,
                    _ => false,
                }
            }
            (TypeCheck::Unit, Value::Unit) => true,
            _ => false,
        };

        self.stack.push(is_match);
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_match_object(&mut self, slot: usize, exact: bool) -> VmResult<()> {
        let value = vm_try!(self.stack.pop());

        let is_match = match value {
            Value::Object(object) => {
                let keys = vm_try!(self
                    .unit
                    .lookup_object_keys(slot)
                    .ok_or(VmErrorKind::MissingStaticObjectKeys { slot }));

                let object = vm_try!(object.borrow_ref());
                test(&object, keys, exact)
            }
            _ => false,
        };

        self.stack.push(is_match);
        return VmResult::Ok(());

        fn test(object: &Object, keys: &[String], exact: bool) -> bool {
            if exact {
                if object.len() != keys.len() {
                    return false;
                }
            } else if object.len() < keys.len() {
                return false;
            }

            for key in keys {
                if !object.contains_key(key) {
                    return false;
                }
            }

            true
        }
    }

    /// Push the given variant onto the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_variant(&mut self, variant: InstVariant) -> VmResult<()> {
        match variant {
            InstVariant::Some => {
                let some = vm_try!(self.stack.pop());
                self.stack.push(Value::Option(Shared::new(Some(some))));
            }
            InstVariant::None => {
                self.stack.push(Value::Option(Shared::new(None)));
            }
            InstVariant::Ok => {
                let some = vm_try!(self.stack.pop());
                self.stack
                    .push(Value::Result(Shared::new(Result::Ok(some))));
            }
            InstVariant::Err => {
                let some = vm_try!(self.stack.pop());
                self.stack
                    .push(Value::Result(Shared::new(Result::Err(some))));
            }
        }

        VmResult::Ok(())
    }

    /// Load a function as a value onto the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_load_fn(&mut self, hash: Hash) -> VmResult<()> {
        let function = vm_try!(self.lookup_function_by_hash(hash));
        self.stack.push(Value::Function(Shared::new(function)));
        VmResult::Ok(())
    }

    /// Construct a closure on the top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_closure(&mut self, hash: Hash, count: usize) -> VmResult<()> {
        let info = vm_try!(self
            .unit
            .function(hash)
            .ok_or(VmErrorKind::MissingFunction { hash }));

        let (offset, call, args) = match info {
            UnitFn::Offset { offset, call, args } => (offset, call, args),
            _ => return err(VmErrorKind::MissingFunction { hash }),
        };

        let environment = vm_try!(self.stack.pop_sequence(count)).into_boxed_slice();

        let function = Function::from_vm_closure(
            self.context.clone(),
            self.unit.clone(),
            offset,
            call,
            args,
            environment,
            hash,
        );

        self.stack.push(Value::Function(Shared::new(function)));
        VmResult::Ok(())
    }

    /// Implementation of a function call.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_call(&mut self, hash: Hash, args: usize) -> VmResult<()> {
        match self.unit.function(hash) {
            Some(info) => match info {
                UnitFn::Offset {
                    offset,
                    call,
                    args: expected,
                } => {
                    vm_try!(check_args(args, expected));
                    vm_try!(self.call_offset_fn(offset, call, args));
                }
                UnitFn::UnitStruct { hash } => {
                    vm_try!(check_args(args, 0));

                    let rtti = vm_try!(self
                        .unit
                        .lookup_rtti(hash)
                        .ok_or(VmErrorKind::MissingRtti { hash }));

                    self.stack.push(Value::unit_struct(rtti.clone()));
                }
                UnitFn::TupleStruct {
                    hash,
                    args: expected,
                } => {
                    vm_try!(check_args(args, expected));
                    let tuple = vm_try!(self.stack.pop_sequence(args));

                    let rtti = vm_try!(self
                        .unit
                        .lookup_rtti(hash)
                        .ok_or(VmErrorKind::MissingRtti { hash }));

                    self.stack.push(Value::tuple_struct(rtti.clone(), tuple));
                }
                UnitFn::TupleVariant {
                    hash,
                    args: expected,
                } => {
                    vm_try!(check_args(args, expected));

                    let rtti = vm_try!(self
                        .unit
                        .lookup_variant_rtti(hash)
                        .ok_or(VmErrorKind::MissingVariantRtti { hash }));

                    let tuple = vm_try!(self.stack.pop_sequence(args));
                    self.stack.push(Value::tuple_variant(rtti.clone(), tuple));
                }
                UnitFn::UnitVariant { hash } => {
                    vm_try!(check_args(args, 0));

                    let rtti = vm_try!(self
                        .unit
                        .lookup_variant_rtti(hash)
                        .ok_or(VmErrorKind::MissingVariantRtti { hash }));

                    self.stack.push(Value::unit_variant(rtti.clone()));
                }
            },
            None => {
                let handler = vm_try!(self
                    .context
                    .function(hash)
                    .ok_or(VmErrorKind::MissingFunction { hash }));

                vm_try!(handler(&mut self.stack, args));
            }
        }

        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_call_instance(&mut self, hash: Hash, args: usize) -> VmResult<()> {
        // NB: +1 to include the instance itself.
        let args = args + 1;
        let instance = vm_try!(self.stack.at_offset_from_top(args));
        let type_hash = vm_try!(instance.type_hash());
        let hash = Hash::associated_function(type_hash, hash);

        if let Some(UnitFn::Offset {
            offset,
            call,
            args: expected,
        }) = self.unit.function(hash)
        {
            vm_try!(check_args(args, expected));
            vm_try!(self.call_offset_fn(offset, call, args));
            return VmResult::Ok(());
        }

        if let Some(handler) = self.context.function(hash) {
            vm_try!(handler(&mut self.stack, args));
            return VmResult::Ok(());
        }

        err(VmErrorKind::MissingInstanceFunction {
            instance: vm_try!(instance.type_info()),
            hash,
        })
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_call_fn(&mut self, args: usize) -> VmResult<Option<VmHalt>> {
        let function = vm_try!(self.stack.pop());

        let ty = match function {
            Value::Type(ty) => ty,
            Value::Function(function) => {
                let function = vm_try!(function.into_ref());
                return function.call_with_vm(self, args);
            }
            actual => {
                let actual = vm_try!(actual.type_info());
                return err(VmErrorKind::UnsupportedCallFn { actual });
            }
        };

        vm_try!(self.op_call(ty.into_hash(), args));
        VmResult::Ok(None)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_iter_next(&mut self, offset: usize, jump: usize) -> VmResult<()> {
        let value = vm_try!(self.stack.at_offset_mut(offset));

        let some = match value {
            Value::Option(option) => {
                let option = vm_try!(option.borrow_ref()).clone();

                match option {
                    Some(some) => some,
                    None => {
                        self.ip = vm_try!(self.unit.translate(jump));
                        return VmResult::Ok(());
                    }
                }
            }
            other => {
                return err(VmErrorKind::UnsupportedIterNextOperand {
                    actual: vm_try!(other.type_info()),
                });
            }
        };

        *value = some;
        VmResult::Ok(())
    }

    /// Call the provided closure within the context of this virtual machine.
    ///
    /// This allows for calling protocol function helpers like
    /// [Value::string_display] which requires access to a virtual machine.
    ///
    /// ```,no_run
    /// use rune::{Context, Unit};
    /// use std::sync::Arc;
    ///
    /// let context = Context::with_default_modules()?;
    /// let context = Arc::new(context.runtime());
    ///
    /// // Normally the unit would be created by compiling some source,
    /// // and since this one is empty it'll just error.
    /// let unit = Arc::new(Unit::default());
    ///
    /// let mut vm = rune::Vm::new(context, unit);
    ///
    /// let output = vm.call(["main"], ())?;
    ///
    /// // Call the string_display protocol on `output`. This requires
    /// // access to a virtual machine since it might use functions
    /// // registered in the unit associated with it.
    /// let mut s = String::new();
    /// let mut buf = String::new();
    ///
    /// // Note: We do an extra unwrap because the return value is
    /// // `fmt::Result`.
    /// vm.with(|| output.string_display(&mut s, &mut buf)).into_result()?.expect("formatting should succeed");
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn with<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let _guard = crate::runtime::env::Guard::new(&self.context, &self.unit);
        f()
    }

    /// Evaluate a single instruction.
    pub(crate) fn run(&mut self) -> VmResult<VmHalt> {
        // NB: set up environment so that native function can access context and
        // unit.
        let _guard = crate::runtime::env::Guard::new(&self.context, &self.unit);

        loop {
            if !budget::take() {
                return VmResult::Ok(VmHalt::Limited);
            }

            let Some((inst, inst_len)) = vm_try!(self.unit.instruction_at(self.ip)) else {
                return VmResult::err(VmErrorKind::IpOutOfBounds {
                    ip: self.ip,
                    length: self.unit.instructions().end(),
                });
            };

            tracing::trace!(ip = ?self.ip, ?inst);

            self.ip = self.ip.wrapping_add(inst_len);

            match inst {
                Inst::Not => {
                    vm_try!(self.op_not());
                }
                Inst::Neg => {
                    vm_try!(self.op_neg());
                }
                Inst::Closure { hash, count } => {
                    vm_try!(self.op_closure(hash, count));
                }
                Inst::Call { hash, args } => {
                    vm_try!(self.op_call(hash, args));
                }
                Inst::CallInstance { hash, args } => {
                    vm_try!(self.op_call_instance(hash, args));
                }
                Inst::CallFn { args } => {
                    if let Some(reason) = vm_try!(self.op_call_fn(args)) {
                        return VmResult::Ok(reason);
                    }
                }
                Inst::LoadInstanceFn { hash } => {
                    vm_try!(self.op_load_instance_fn(hash));
                }
                Inst::IndexGet { target, index } => {
                    vm_try!(self.op_index_get(target, index));
                }
                Inst::TupleIndexGet { index } => {
                    vm_try!(self.op_tuple_index_get(index));
                }
                Inst::TupleIndexSet { index } => {
                    vm_try!(self.op_tuple_index_set(index));
                }
                Inst::TupleIndexGetAt { offset, index } => {
                    vm_try!(self.op_tuple_index_get_at(offset, index));
                }
                Inst::ObjectIndexGet { slot } => {
                    vm_try!(self.op_object_index_get(slot));
                }
                Inst::ObjectIndexSet { slot } => {
                    vm_try!(self.op_object_index_set(slot));
                }
                Inst::ObjectIndexGetAt { offset, slot } => {
                    vm_try!(self.op_object_index_get_at(offset, slot));
                }
                Inst::IndexSet => {
                    vm_try!(self.op_index_set());
                }
                Inst::Return { address, clean } => {
                    if vm_try!(self.op_return(address, clean)) {
                        return VmResult::Ok(VmHalt::Exited);
                    }
                }
                Inst::ReturnUnit => {
                    if vm_try!(self.op_return_unit()) {
                        return VmResult::Ok(VmHalt::Exited);
                    }
                }
                Inst::Await => {
                    let future = vm_try!(self.op_await());
                    // NB: the future itself will advance the virtual machine.
                    return VmResult::Ok(VmHalt::Awaited(Awaited::Future(future)));
                }
                Inst::Select { len } => {
                    if let Some(select) = vm_try!(self.op_select(len)) {
                        // NB: the future itself will advance the virtual machine.
                        return VmResult::Ok(VmHalt::Awaited(Awaited::Select(select)));
                    }
                }
                Inst::LoadFn { hash } => {
                    vm_try!(self.op_load_fn(hash));
                }
                Inst::Push { value } => {
                    vm_try!(self.op_push(value));
                }
                Inst::Pop => {
                    vm_try!(self.op_pop());
                }
                Inst::PopN { count } => {
                    vm_try!(self.op_popn(count));
                }
                Inst::PopAndJumpIfNot { count, jump } => {
                    vm_try!(self.op_pop_and_jump_if_not(count, jump));
                }
                Inst::Clean { count } => {
                    vm_try!(self.op_clean(count));
                }
                Inst::Copy { offset } => {
                    vm_try!(self.op_copy(offset));
                }
                Inst::Move { offset } => {
                    vm_try!(self.op_move(offset));
                }
                Inst::Drop { offset } => {
                    vm_try!(self.op_drop(offset));
                }
                Inst::Dup => {
                    vm_try!(self.op_dup());
                }
                Inst::Replace { offset } => {
                    vm_try!(self.op_replace(offset));
                }
                Inst::Jump { jump } => {
                    vm_try!(self.op_jump(jump));
                }
                Inst::JumpIf { jump } => {
                    vm_try!(self.op_jump_if(jump));
                }
                Inst::JumpIfOrPop { jump } => {
                    vm_try!(self.op_jump_if_or_pop(jump));
                }
                Inst::JumpIfNotOrPop { jump } => {
                    vm_try!(self.op_jump_if_not_or_pop(jump));
                }
                Inst::JumpIfBranch { branch, jump } => {
                    vm_try!(self.op_jump_if_branch(branch, jump));
                }
                Inst::Vec { count } => {
                    vm_try!(self.op_vec(count));
                }
                Inst::Tuple { count } => {
                    vm_try!(self.op_tuple(count));
                }
                Inst::Tuple1 { args } => {
                    vm_try!(self.op_tuple_n(&args[..]));
                }
                Inst::Tuple2 { args } => {
                    vm_try!(self.op_tuple_n(&args[..]));
                }
                Inst::Tuple3 { args } => {
                    vm_try!(self.op_tuple_n(&args[..]));
                }
                Inst::Tuple4 { args } => {
                    vm_try!(self.op_tuple_n(&args[..]));
                }
                Inst::PushTuple => {
                    vm_try!(self.op_push_tuple());
                }
                Inst::Object { slot } => {
                    vm_try!(self.op_object(slot));
                }
                Inst::Range { limits } => {
                    vm_try!(self.op_range(limits));
                }
                Inst::UnitStruct { hash } => {
                    vm_try!(self.op_empty_struct(hash));
                }
                Inst::Struct { hash, slot } => {
                    vm_try!(self.op_struct(hash, slot));
                }
                Inst::UnitVariant { hash } => {
                    vm_try!(self.op_unit_variant(hash));
                }
                Inst::StructVariant { hash, slot } => {
                    vm_try!(self.op_object_variant(hash, slot));
                }
                Inst::String { slot } => {
                    vm_try!(self.op_string(slot));
                }
                Inst::Bytes { slot } => {
                    vm_try!(self.op_bytes(slot));
                }
                Inst::StringConcat { len, size_hint } => {
                    vm_try!(self.op_string_concat(len, size_hint));
                }
                Inst::Format { spec } => {
                    vm_try!(self.op_format(spec));
                }
                Inst::IsUnit => {
                    vm_try!(self.op_is_unit());
                }
                Inst::Try {
                    address,
                    clean,
                    preserve,
                } => {
                    if vm_try!(self.op_try(address, clean, preserve)) {
                        return VmResult::Ok(VmHalt::Exited);
                    }
                }
                Inst::EqByte { byte } => {
                    vm_try!(self.op_eq_byte(byte));
                }
                Inst::EqChar { char: character } => {
                    vm_try!(self.op_eq_character(character));
                }
                Inst::EqInteger { integer } => {
                    vm_try!(self.op_eq_integer(integer));
                }
                Inst::EqBool { boolean } => {
                    vm_try!(self.op_eq_bool(boolean));
                }
                Inst::EqString { slot } => {
                    vm_try!(self.op_eq_string(slot));
                }
                Inst::EqBytes { slot } => {
                    vm_try!(self.op_eq_bytes(slot));
                }
                Inst::MatchSequence {
                    type_check,
                    len,
                    exact,
                } => {
                    vm_try!(self.op_match_sequence(type_check, len, exact));
                }
                Inst::MatchType { hash } => {
                    vm_try!(self.op_match_type(hash));
                }
                Inst::MatchVariant {
                    enum_hash,
                    variant_hash,
                    index,
                } => {
                    vm_try!(self.op_match_variant(enum_hash, variant_hash, index));
                }
                Inst::MatchBuiltIn { type_check } => {
                    vm_try!(self.op_match_builtin(type_check));
                }
                Inst::MatchObject { slot, exact } => {
                    vm_try!(self.op_match_object(slot, exact));
                }
                Inst::Yield => {
                    return VmResult::Ok(VmHalt::Yielded);
                }
                Inst::YieldUnit => {
                    self.stack.push(Value::Unit);
                    return VmResult::Ok(VmHalt::Yielded);
                }
                Inst::Variant { variant } => {
                    vm_try!(self.op_variant(variant));
                }
                Inst::Op { op, a, b } => {
                    vm_try!(self.op_op(op, a, b));
                }
                Inst::Assign { target, op } => {
                    vm_try!(self.op_assign(target, op));
                }
                Inst::IterNext { offset, jump } => {
                    vm_try!(self.op_iter_next(offset, jump));
                }
                Inst::Panic { reason } => {
                    return err(VmErrorKind::Panic {
                        reason: Panic::from(reason),
                    });
                }
            }
        }
    }
}

impl AsMut<Vm> for Vm {
    #[inline]
    fn as_mut(&mut self) -> &mut Vm {
        self
    }
}

impl AsRef<Vm> for Vm {
    #[inline]
    fn as_ref(&self) -> &Vm {
        self
    }
}

/// A call frame.
///
/// This is used to store the return point after an instruction has been run.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct CallFrame {
    /// The stored instruction pointer.
    pub ip: usize,
    /// The top of the stack at the time of the call to ensure stack isolation
    /// across function calls.
    ///
    /// I.e. a function should not be able to manipulate the size of any other
    /// stack than its own.
    pub stack_bottom: usize,
}

/// Clear stack on drop.
struct ClearStack<'a>(&'a mut Vm);

impl Drop for ClearStack<'_> {
    fn drop(&mut self) {
        self.0.stack.clear();
    }
}

/// Check that arguments matches expected or raise the appropriate error.
fn check_args(args: usize, expected: usize) -> Result<(), VmErrorKind> {
    if args != expected {
        return Err(VmErrorKind::BadArgumentCount {
            actual: args,
            expected,
        });
    }

    Result::Ok(())
}
