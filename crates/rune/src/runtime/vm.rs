use core::cmp::Ordering;
use core::mem::replace;
use core::ops;
use core::ptr::NonNull;
use core::slice;

use ::rust_alloc::sync::Arc;

use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::hash::{Hash, IntoHash, ToTypeHash};
use crate::modules::{option, result};
use crate::runtime::budget;
use crate::runtime::future::SelectFuture;
use crate::runtime::unit::{UnitFn, UnitStorage};
use crate::runtime::{
    self, Args, Awaited, BorrowMut, Bytes, Call, ControlFlow, EmptyStruct, Format, FormatSpec,
    Formatter, FromValue, Function, Future, Generator, GuardedArgs, Inst, InstAddress,
    InstAssignOp, InstOp, InstRange, InstTarget, InstValue, InstVariant, Object, Output,
    OwnedTuple, Panic, Protocol, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
    RangeToInclusive, RuntimeContext, Select, Stack, Stream, Struct, Type, TypeCheck, TypeOf, Unit,
    Value, ValueKind, Variant, VariantData, Vec, VmError, VmErrorKind, VmExecution, VmHalt,
    VmIntegerRepr, VmResult, VmSendExecution,
};

use super::{VmDiagnostics, VmDiagnosticsObj};

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
    Value(&'a Value, Value),
    /// Fallback to a different kind of operation.
    Fallback(TargetFallback<'b>),
}

macro_rules! target_value {
    ($vm:ident, $target:expr, $guard:ident, $lhs:ident, $rhs:ident) => {{
        let rhs = vm_try!($vm.stack.at($rhs)).clone();

        match $target {
            InstTarget::Address(addr) => TargetValue::Value(vm_try!($vm.stack.at(addr)), rhs),
            InstTarget::TupleField(lhs, index) => {
                $lhs = vm_try!($vm.stack.at(lhs)).clone();

                if let Some(value) = vm_try!(Vm::try_tuple_like_index_get_mut(&$lhs, index)) {
                    $guard = value;
                    TargetValue::Value(&mut *$guard, rhs)
                } else {
                    TargetValue::Fallback(TargetFallback::Index(&$lhs, index, rhs))
                }
            }
            InstTarget::Field(lhs, field) => {
                let field = vm_try!($vm.unit.lookup_string(field));
                $lhs = vm_try!($vm.stack.at(lhs)).clone();

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
#[derive(Debug)]
pub struct Vm {
    /// Context associated with virtual machine.
    context: Arc<RuntimeContext>,
    /// Unit associated with virtual machine.
    unit: Arc<Unit>,
    /// The current instruction pointer.
    ip: usize,
    /// The length of the last instruction pointer.
    last_ip_len: u8,
    /// The current stack.
    stack: Stack,
    /// Frames relative to the stack.
    call_frames: alloc::Vec<CallFrame>,
}

impl Vm {
    /// Construct a new virtual machine.
    ///
    /// Constructing a virtual machine is a cheap constant-time operation.
    ///
    /// See [`unit_mut`] and [`context_mut`] documentation for information on
    /// how to re-use existing [`Vm`]'s.
    ///
    /// [`unit_mut`]: Vm::unit_mut
    /// [`context_mut`]: Vm::context_mut
    pub const fn new(context: Arc<RuntimeContext>, unit: Arc<Unit>) -> Self {
        Self::with_stack(context, unit, Stack::new())
    }

    /// Construct a new virtual machine with a custom stack.
    pub const fn with_stack(context: Arc<RuntimeContext>, unit: Arc<Unit>, stack: Stack) -> Self {
        Self {
            context,
            unit,
            ip: 0,
            last_ip_len: 0,
            stack,
            call_frames: alloc::Vec::new(),
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

    /// Test if the virtual machine is the same context.
    pub fn is_same_context(&self, context: &Arc<RuntimeContext>) -> bool {
        Arc::ptr_eq(&self.context, context)
    }

    /// Test if the virtual machine is the same context.
    pub fn is_same_unit(&self, unit: &Arc<Unit>) -> bool {
        Arc::ptr_eq(&self.unit, unit)
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

    /// Access the context related to the virtual machine mutably.
    ///
    /// Note that this can be used to swap out the [`RuntimeContext`] associated
    /// with the running vm. Note that this is only necessary if the underlying
    /// [`Context`] is different or has been modified. In contrast to
    /// constructing a [`new`] vm, this allows for amortised re-use of any
    /// allocations.
    ///
    /// After doing this, it's important to call [`clear`] to clean up any
    /// residual state.
    ///
    /// [`clear`]: Vm::clear
    /// [`Context`]: crate::Context
    /// [`new`]: Vm::new
    #[inline]
    pub fn context_mut(&mut self) -> &mut Arc<RuntimeContext> {
        &mut self.context
    }

    /// Access the context related to the virtual machine.
    #[inline]
    pub fn context(&self) -> &Arc<RuntimeContext> {
        &self.context
    }

    /// Access the underlying unit of the virtual machine mutably.
    ///
    /// Note that this can be used to swap out the [`Unit`] of execution in the
    /// running vm. In contrast to constructing a [`new`] vm, this allows for
    /// amortised re-use of any allocations.
    ///
    /// After doing this, it's important to call [`clear`] to clean up any
    /// residual state.
    ///
    /// [`clear`]: Vm::clear
    /// [`new`]: Vm::new
    #[inline]
    pub fn unit_mut(&mut self) -> &mut Arc<Unit> {
        &mut self.unit
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

    /// Access the last instruction that was executed.
    #[inline]
    pub fn last_ip(&self) -> usize {
        self.ip.wrapping_sub(self.last_ip_len as usize)
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
    /// let context = Arc::new(context.runtime()?);
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
    /// let value: i64 = dynamic_max.call::<i64>((10, 20)).into_result()?;
    /// assert_eq!(value, 20);
    ///
    /// // Building an item buffer to lookup an `::std` item.
    /// let mut item = ItemBuf::with_crate("std")?;
    /// item.push("i64")?;
    /// item.push("max")?;
    ///
    /// let max = vm.lookup_function(&item)?;
    ///
    /// let value: i64 = max.call::<i64>((10, 20)).into_result()?;
    /// assert_eq!(value, 20);
    /// # Ok::<_, rune::support::Error>(())
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
    /// let context = Arc::new(context.runtime()?);
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
    /// # Ok::<_, rune::support::Error>(())
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
    /// let context = Arc::new(context.runtime()?);
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
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn execute<A, N>(&mut self, name: N, args: A) -> Result<VmExecution<&mut Self>, VmError>
    where
        N: ToTypeHash,
        A: Args,
    {
        self.set_entrypoint(name, args.count())?;
        args.into_stack(&mut self.stack).into_result()?;
        Ok(VmExecution::new(self))
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
        Ok(VmSendExecution(VmExecution::new(self)))
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
    pub fn call(
        &mut self,
        name: impl ToTypeHash,
        args: impl GuardedArgs,
    ) -> Result<Value, VmError> {
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
        Ok(value)
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
    pub fn call_with_diagnostics(
        &mut self,
        name: impl ToTypeHash,
        args: impl GuardedArgs,
        diagnostics: Option<&mut dyn VmDiagnostics>,
    ) -> Result<Value, VmError> {
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
                .complete_with_diagnostics(diagnostics)
                .into_result()?
        };

        // Note: this might panic if something in the vm is holding on to a
        // reference of the value. We should prevent it from being possible to
        // take any owned references to values held by this.
        drop(guard);
        Ok(value)
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
        Ok(value)
    }

    /// Update the instruction pointer to match the function matching the given
    /// name and check that the number of argument matches.
    fn set_entrypoint<N>(&mut self, name: N, count: usize) -> Result<(), VmErrorKind>
    where
        N: ToTypeHash,
    {
        let hash = name.to_type_hash();

        let Some(info) = self.unit.function(hash) else {
            return Err(if let Some(item) = name.to_item()? {
                VmErrorKind::MissingEntry { hash, item }
            } else {
                VmErrorKind::MissingEntryHash { hash }
            });
        };

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
        out: Output,
    ) -> VmResult<CallResult<()>>
    where
        H: ToTypeHash,
        A: GuardedArgs,
    {
        let count = args.count().wrapping_add(1);
        let type_hash = vm_try!(target.type_hash());
        let hash = Hash::associated_function(type_hash, hash.to_type_hash());

        if let Some(UnitFn::Offset {
            offset,
            call,
            args: expected,
            ..
        }) = self.unit.function(hash)
        {
            let addr = self.stack.addr();
            vm_try!(self.stack.push(target));
            // Safety: We hold onto the guard for the duration of this call.
            let _guard = unsafe { vm_try!(args.unsafe_into_stack(&mut self.stack)) };
            vm_try!(check_args(count, expected));
            vm_try!(self.call_offset_fn(offset, call, addr, count, out));
            return VmResult::Ok(CallResult::Ok(()));
        }

        if let Some(handler) = self.context.function(hash) {
            let addr = self.stack.addr();
            vm_try!(self.called_function_hook(hash));
            vm_try!(self.stack.push(target));
            // Safety: We hold onto the guard for the duration of this call.
            let _guard = unsafe { vm_try!(args.unsafe_into_stack(&mut self.stack)) };
            vm_try!(handler(&mut self.stack, addr, count, out));
            return VmResult::Ok(CallResult::Ok(()));
        }

        VmResult::Ok(CallResult::Unsupported(target))
    }

    fn called_function_hook(&self, hash: Hash) -> VmResult<()> {
        crate::runtime::env::exclusive(|_, _, diagnostics| {
            if let Some(diagnostics) = diagnostics {
                vm_try!(diagnostics.function_used(hash, self.ip()));
            }

            VmResult::Ok(())
        })
    }

    /// Helper to call a field function.
    #[inline(always)]
    fn call_field_fn<N, A>(
        &mut self,
        protocol: Protocol,
        target: Value,
        name: N,
        args: A,
        out: Output,
    ) -> VmResult<CallResult<()>>
    where
        N: IntoHash,
        A: GuardedArgs,
    {
        let count = args.count().wrapping_add(1);
        let hash = Hash::field_function(protocol, vm_try!(target.type_hash()), name);

        if let Some(handler) = self.context.function(hash) {
            let addr = self.stack.addr();
            vm_try!(self.called_function_hook(hash));
            vm_try!(self.stack.push(target));
            let _guard = unsafe { vm_try!(args.unsafe_into_stack(&mut self.stack)) };
            vm_try!(handler(&mut self.stack, addr, count, out));
            return VmResult::Ok(CallResult::Ok(()));
        }

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
        out: Output,
    ) -> VmResult<CallResult<()>>
    where
        A: GuardedArgs,
    {
        let count = args.count().wrapping_add(1);
        let hash = Hash::index_function(protocol, vm_try!(target.type_hash()), Hash::index(index));

        if let Some(handler) = self.context.function(hash) {
            let addr = self.stack.addr();
            vm_try!(self.stack.push(target));
            let _guard = unsafe { vm_try!(args.unsafe_into_stack(&mut self.stack)) };
            vm_try!(handler(&mut self.stack, addr, count, out));
            return VmResult::Ok(CallResult::Ok(()));
        }

        VmResult::Ok(CallResult::Unsupported(target))
    }

    fn internal_boolean_ops(
        &mut self,
        match_ordering: fn(Ordering) -> bool,
        lhs: InstAddress,
        rhs: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.at(rhs)).clone();
        let lhs = vm_try!(self.stack.at(lhs)).clone();

        let value = match vm_try!(Value::partial_cmp_with(&lhs, &rhs, self)) {
            Some(ordering) => match_ordering(ordering),
            None => false,
        };

        vm_try!(out.store(&mut self.stack, || VmResult::Ok(value)));
        VmResult::Ok(())
    }

    /// Push a new call frame.
    ///
    /// This will cause the `args` number of elements on the stack to be
    /// associated and accessible to the new call frame.
    #[tracing::instrument(skip(self), fields(call_frames = self.call_frames.len(), stack_bottom = self.stack.stack_bottom(), stack = self.stack.len(), self.ip))]
    pub(crate) fn push_call_frame(
        &mut self,
        ip: usize,
        addr: InstAddress,
        args: usize,
        isolated: bool,
        out: Output,
    ) -> Result<(), VmErrorKind> {
        tracing::trace!("pushing call frame");

        let stack_bottom = self.stack.swap_stack_bottom(addr, args)?;
        let ip = replace(&mut self.ip, ip);

        let frame = CallFrame {
            ip,
            stack_bottom,
            isolated,
            out,
        };

        self.call_frames.try_push(frame)?;
        Ok(())
    }

    /// Pop a call frame from an internal call, which needs the current stack
    /// pointer to be returned and does not check for context isolation through
    /// [`CallFrame::isolated`].
    #[tracing::instrument(skip(self), fields(call_frames = self.call_frames.len(), stack_bottom = self.stack.stack_bottom(), stack = self.stack.len(), self.ip))]
    pub(crate) fn pop_call_frame_from_call(&mut self) -> Result<Option<usize>, VmErrorKind> {
        tracing::trace!("popping call frame from call");

        let Some(frame) = self.call_frames.pop() else {
            return Ok(None);
        };

        tracing::trace!(?frame);
        self.stack.pop_stack_top(frame.stack_bottom)?;
        Ok(Some(replace(&mut self.ip, frame.ip)))
    }

    /// Pop a call frame and return it.
    #[tracing::instrument(skip(self), fields(call_frames = self.call_frames.len(), stack_bottom = self.stack.stack_bottom(), stack = self.stack.len(), self.ip))]
    pub(crate) fn pop_call_frame(&mut self) -> Result<(bool, Output), VmErrorKind> {
        tracing::trace!("popping call frame");

        let Some(frame) = self.call_frames.pop() else {
            self.stack.pop_stack_top(0)?;
            self.stack.push(())?;
            return Ok((true, Output::keep(0)));
        };

        tracing::trace!(?frame);
        self.stack.pop_stack_top(frame.stack_bottom)?;
        self.ip = frame.ip;
        Ok((frame.isolated, frame.out))
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_object_like_index_get(target: &Value, field: &str) -> VmResult<Option<Value>> {
        let target = vm_try!(target.borrow_kind_ref());

        let value = match &*target {
            ValueKind::Object(ref target) => target.get(field),
            ValueKind::Struct(ref target) => target.get(field),
            ValueKind::Variant(ref variant) => match variant.data() {
                VariantData::Struct(target) => target.get(field),
                _ => return VmResult::Ok(None),
            },
            _ => return VmResult::Ok(None),
        };

        let Some(value) = value else {
            return err(VmErrorKind::MissingField {
                target: target.type_info(),
                field: vm_try!(field.try_to_owned()),
            });
        };

        VmResult::Ok(Some(value.clone()))
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_tuple_like_index_get(target: &Value, index: usize) -> VmResult<Option<Value>> {
        use crate::runtime::GeneratorState::*;

        let target = vm_try!(target.borrow_kind_ref());

        let value = match &*target {
            ValueKind::EmptyTuple => None,
            ValueKind::Tuple(tuple) => tuple.get(index),
            ValueKind::Vec(vec) => vec.get(index),
            ValueKind::Result(result) => match (index, result) {
                (0, Ok(value)) => Some(value),
                (0, Err(value)) => Some(value),
                _ => None,
            },
            ValueKind::Option(option) => match (index, option) {
                (0, Some(value)) => Some(value),
                _ => None,
            },
            ValueKind::GeneratorState(state) => match (index, state) {
                (0, Yielded(value)) => Some(value),
                (0, Complete(value)) => Some(value),
                _ => None,
            },
            ValueKind::TupleStruct(tuple_struct) => tuple_struct.data().get(index),
            ValueKind::Variant(variant) => match variant.data() {
                VariantData::Tuple(tuple) => tuple.get(index),
                _ => return VmResult::Ok(None),
            },
            _ => return VmResult::Ok(None),
        };

        let Some(value) = value else {
            return err(VmErrorKind::MissingIndexInteger {
                target: target.type_info(),
                index: VmIntegerRepr::from(index),
            });
        };

        VmResult::Ok(Some(value.clone()))
    }

    /// Implementation of getting a mutable value out of a tuple-like value.
    fn try_tuple_like_index_get_mut(
        target: &Value,
        index: usize,
    ) -> VmResult<Option<BorrowMut<'_, Value>>> {
        use crate::runtime::GeneratorState::*;

        let mut unsupported = false;

        let result = BorrowMut::try_map(vm_try!(target.borrow_kind_mut()), |kind| {
            match kind {
                ValueKind::Tuple(tuple) => return tuple.get_mut(index),
                ValueKind::Vec(vec) => return vec.get_mut(index),
                ValueKind::Result(result) => match (index, result) {
                    (0, Ok(value)) => return Some(value),
                    (0, Err(value)) => return Some(value),
                    _ => {}
                },
                ValueKind::Option(option) => {
                    if let (0, Some(value)) = (index, option) {
                        return Some(value);
                    }
                }
                ValueKind::GeneratorState(state) => match (index, state) {
                    (0, Yielded(value)) => return Some(value),
                    (0, Complete(value)) => return Some(value),
                    _ => {}
                },
                ValueKind::TupleStruct(tuple_struct) => return tuple_struct.get_mut(index),
                ValueKind::Variant(Variant {
                    data: VariantData::Tuple(tuple),
                    ..
                }) => {
                    return tuple.get_mut(index);
                }
                _ => {}
            }

            unsupported = true;
            None
        });

        if unsupported {
            return VmResult::Ok(None);
        }

        match result {
            Ok(value) => VmResult::Ok(Some(value)),
            Err(actual) => err(VmErrorKind::MissingIndexInteger {
                target: actual.type_info(),
                index: VmIntegerRepr::from(index),
            }),
        }
    }

    /// Implementation of getting a mutable string index on an object-like type.
    fn try_object_like_index_get_mut<'a>(
        target: &'a Value,
        field: &str,
    ) -> VmResult<Option<BorrowMut<'a, Value>>> {
        let mut unsupported = false;

        let result = BorrowMut::try_map(vm_try!(target.borrow_kind_mut()), |kind| {
            match kind {
                ValueKind::Object(target) => {
                    return target.get_mut(field);
                }
                ValueKind::Struct(target) => {
                    return target.get_mut(field);
                }
                ValueKind::Variant(Variant {
                    data: VariantData::Struct(st),
                    ..
                }) => {
                    return st.get_mut(field);
                }
                _ => {}
            }

            unsupported = true;
            None
        });

        if unsupported {
            return VmResult::Ok(None);
        }

        match result {
            Ok(value) => VmResult::Ok(Some(value)),
            Err(actual) => err(VmErrorKind::MissingField {
                target: actual.type_info(),
                field: vm_try!(field.try_to_owned()),
            }),
        }
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_tuple_like_index_set(target: &Value, index: usize, value: Value) -> VmResult<bool> {
        match &mut *vm_try!(target.borrow_kind_mut()) {
            ValueKind::EmptyTuple => VmResult::Ok(false),
            ValueKind::Tuple(tuple) => {
                if let Some(target) = tuple.get_mut(index) {
                    *target = value;
                    return VmResult::Ok(true);
                }

                VmResult::Ok(false)
            }
            ValueKind::Vec(vec) => {
                if let Some(target) = vec.get_mut(index) {
                    *target = value;
                    return VmResult::Ok(true);
                }

                VmResult::Ok(false)
            }
            ValueKind::Result(result) => {
                let target = match result {
                    Ok(ok) if index == 0 => ok,
                    Err(err) if index == 1 => err,
                    _ => return VmResult::Ok(false),
                };

                *target = value;
                VmResult::Ok(true)
            }
            ValueKind::Option(option) => {
                let target = match option {
                    Some(some) if index == 0 => some,
                    _ => return VmResult::Ok(false),
                };

                *target = value;
                VmResult::Ok(true)
            }
            ValueKind::TupleStruct(tuple_struct) => {
                if let Some(target) = tuple_struct.get_mut(index) {
                    *target = value;
                    return VmResult::Ok(true);
                }

                VmResult::Ok(false)
            }
            ValueKind::Variant(variant) => {
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
        slot: usize,
        out: Output,
    ) -> VmResult<CallResult<()>> {
        let index = vm_try!(self.unit.lookup_string(slot));

        'out: {
            match &*vm_try!(target.borrow_kind_ref()) {
                ValueKind::Object(object) => {
                    if let Some(value) = object.get(index.as_str()) {
                        vm_try!(out.store(&mut self.stack, || value.clone()));
                        return VmResult::Ok(CallResult::Ok(()));
                    }
                }
                ValueKind::Struct(typed_object) => {
                    if let Some(value) = typed_object.get(index.as_str()) {
                        vm_try!(out.store(&mut self.stack, || value.clone()));
                        return VmResult::Ok(CallResult::Ok(()));
                    }
                }
                ValueKind::Variant(Variant {
                    data: VariantData::Struct(data),
                    ..
                }) => {
                    if let Some(value) = data.get(index.as_str()) {
                        vm_try!(out.store(&mut self.stack, || value.clone()));
                        return VmResult::Ok(CallResult::Ok(()));
                    }
                }
                _ => {
                    break 'out;
                }
            }

            return err(VmErrorKind::ObjectIndexMissing { slot });
        };

        let hash = index.hash();

        VmResult::Ok(
            match vm_try!(self.call_field_fn(Protocol::GET, target, hash, (), out)) {
                CallResult::Ok(()) => CallResult::Ok(()),
                CallResult::Unsupported(target) => CallResult::Unsupported(target),
            },
        )
    }

    fn try_object_slot_index_set(
        &mut self,
        target: Value,
        string_slot: usize,
        value: Value,
    ) -> VmResult<CallResult<()>> {
        let field = vm_try!(self.unit.lookup_string(string_slot));

        match &mut *vm_try!(target.borrow_kind_mut()) {
            ValueKind::Object(object) => {
                let key = vm_try!(field.as_str().try_to_owned());
                vm_try!(object.insert(key, value));
                return VmResult::Ok(CallResult::Ok(()));
            }
            ValueKind::Struct(typed_object) => {
                if let Some(v) = typed_object.get_mut(field.as_str()) {
                    *v = value;
                    return VmResult::Ok(CallResult::Ok(()));
                }

                return err(VmErrorKind::MissingField {
                    target: typed_object.type_info(),
                    field: vm_try!(field.as_str().try_to_owned()),
                });
            }
            ValueKind::Variant(variant) => {
                if let VariantData::Struct(data) = variant.data_mut() {
                    if let Some(v) = data.get_mut(field.as_str()) {
                        *v = value;
                        return VmResult::Ok(CallResult::Ok(()));
                    }
                }

                return err(VmErrorKind::MissingField {
                    target: variant.type_info(),
                    field: vm_try!(field.as_str().try_to_owned()),
                });
            }
            _ => {}
        }

        let hash = field.hash();

        let value =
            vm_try!(self.call_field_fn(Protocol::SET, target, hash, (value,), Output::discard()));

        VmResult::Ok(value)
    }

    fn on_tuple<F, O>(&self, ty: TypeCheck, value: &Value, f: F) -> VmResult<Option<O>>
    where
        F: FnOnce(&[Value]) -> O,
    {
        use crate::runtime::GeneratorState::*;

        VmResult::Ok(match (ty, &*vm_try!(value.borrow_kind_ref())) {
            (TypeCheck::EmptyTuple, ValueKind::EmptyTuple) => Some(f(&[])),
            (TypeCheck::Tuple, ValueKind::Tuple(tuple)) => Some(f(tuple)),
            (TypeCheck::Vec, ValueKind::Vec(vec)) => Some(f(vec)),
            (TypeCheck::Result(v), ValueKind::Result(result)) => Some(match (v, result) {
                (0, Ok(ok)) => f(slice::from_ref(ok)),
                (1, Err(err)) => f(slice::from_ref(err)),
                _ => return VmResult::Ok(None),
            }),
            (TypeCheck::Option(v), ValueKind::Option(option)) => Some(match (v, option) {
                (0, Some(some)) => f(slice::from_ref(some)),
                (1, None) => f(&[]),
                _ => return VmResult::Ok(None),
            }),
            (TypeCheck::GeneratorState(v), ValueKind::GeneratorState(state)) => {
                Some(match (v, state) {
                    (0, Complete(complete)) => f(slice::from_ref(complete)),
                    (1, Yielded(yielded)) => f(slice::from_ref(yielded)),
                    _ => return VmResult::Ok(None),
                })
            }
            _ => None,
        })
    }

    /// Internal implementation of the instance check.
    fn as_op(&mut self, lhs: InstAddress, rhs: InstAddress) -> VmResult<Value> {
        let b = vm_try!(self.stack.at(rhs)).clone();
        let a = vm_try!(self.stack.at(lhs)).clone();

        let ValueKind::Type(ty) = *vm_try!(b.borrow_kind_ref()) else {
            return err(VmErrorKind::UnsupportedIs {
                value: vm_try!(a.type_info()),
                test_type: vm_try!(b.type_info()),
            });
        };

        macro_rules! convert {
            ($from:ty, $value:expr, $ty:expr) => {
                match $ty.into_hash() {
                    runtime::static_type::FLOAT_TYPE_HASH => {
                        vm_try!(Value::try_from($value as f64))
                    }
                    runtime::static_type::BYTE_TYPE_HASH => vm_try!(Value::try_from($value as u8)),
                    runtime::static_type::INTEGER_TYPE_HASH => {
                        vm_try!(Value::try_from($value as i64))
                    }
                    ty => {
                        return err(VmErrorKind::UnsupportedAs {
                            value: <$from as TypeOf>::type_info(),
                            type_hash: ty,
                        });
                    }
                }
            };
        }

        let value = match &*vm_try!(a.borrow_kind_ref()) {
            ValueKind::Integer(a) => convert!(i64, *a, ty),
            ValueKind::Float(a) => convert!(f64, *a, ty),
            ValueKind::Byte(a) => convert!(u8, *a, ty),
            kind => {
                return err(VmErrorKind::UnsupportedAs {
                    value: kind.type_info(),
                    type_hash: ty.into_hash(),
                });
            }
        };

        VmResult::Ok(value)
    }

    /// Internal implementation of the instance check.
    fn test_is_instance(&mut self, lhs: InstAddress, rhs: InstAddress) -> VmResult<bool> {
        let b = vm_try!(self.stack.at(rhs)).clone();
        let a = vm_try!(self.stack.at(lhs)).clone();

        let ValueKind::Type(ty) = *vm_try!(b.borrow_kind_ref()) else {
            return err(VmErrorKind::UnsupportedIs {
                value: vm_try!(a.type_info()),
                test_type: vm_try!(b.type_info()),
            });
        };

        VmResult::Ok(vm_try!(a.type_hash()) == ty.into_hash())
    }

    fn internal_boolean_op(
        &mut self,
        bool_op: impl FnOnce(bool, bool) -> bool,
        op: &'static str,
        lhs: InstAddress,
        rhs: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.at(rhs)).clone();
        let lhs = vm_try!(self.stack.at(lhs)).clone();

        match (
            &*vm_try!(lhs.borrow_kind_ref()),
            &*vm_try!(rhs.borrow_kind_ref()),
        ) {
            (ValueKind::Bool(lhs), ValueKind::Bool(rhs)) => {
                vm_try!(out.store(&mut self.stack, || VmResult::Ok(bool_op(*lhs, *rhs))));
            }
            (lhs, rhs) => {
                return err(VmErrorKind::UnsupportedBinaryOperation {
                    op,
                    lhs: lhs.type_info(),
                    rhs: rhs.type_info(),
                });
            }
        };

        VmResult::Ok(())
    }

    /// Construct a future from calling an async function.
    fn call_generator_fn(
        &mut self,
        offset: usize,
        addr: InstAddress,
        args: usize,
        out: Output,
    ) -> Result<(), VmErrorKind> {
        let iter = self.stack.slice_at(addr, args)?;

        if let Some(at) = out.as_addr() {
            let stack = iter.iter().cloned().try_collect::<Stack>()?;
            let mut vm = Self::with_stack(self.context.clone(), self.unit.clone(), stack);
            vm.ip = offset;
            *self.stack.at_mut(at)? = Value::try_from(Generator::new(vm))?;
        }

        Ok(())
    }

    /// Construct a stream from calling a function.
    fn call_stream_fn(
        &mut self,
        offset: usize,
        addr: InstAddress,
        args: usize,
        out: Output,
    ) -> Result<(), VmErrorKind> {
        let stack = self.stack.slice_at(addr, args)?;

        if let Some(at) = out.as_addr() {
            let stack = stack.iter().cloned().try_collect::<Stack>()?;
            let mut vm = Self::with_stack(self.context.clone(), self.unit.clone(), stack);
            vm.ip = offset;
            *self.stack.at_mut(at)? = Value::try_from(Stream::new(vm))?;
        }

        Ok(())
    }

    /// Construct a future from calling a function.
    fn call_async_fn(
        &mut self,
        offset: usize,
        addr: InstAddress,
        args: usize,
        out: Output,
    ) -> Result<(), VmErrorKind> {
        let stack = self.stack.slice_at(addr, args)?;

        if let Some(at) = out.as_addr() {
            let stack = stack.iter().cloned().try_collect::<Stack>()?;
            let mut vm = Self::with_stack(self.context.clone(), self.unit.clone(), stack);
            vm.ip = offset;
            let mut execution = vm.into_execution();
            let future = Future::new(async move { execution.async_complete().await })?;
            *self.stack.at_mut(at)? = Value::try_from(future)?;
        }

        Ok(())
    }

    /// Helper function to call the function at the given offset.
    fn call_offset_fn(
        &mut self,
        offset: usize,
        call: Call,
        addr: InstAddress,
        args: usize,
        out: Output,
    ) -> Result<bool, VmErrorKind> {
        let moved = match call {
            Call::Async => {
                self.call_async_fn(offset, addr, args, out)?;
                false
            }
            Call::Immediate => {
                self.push_call_frame(offset, addr, args, false, out)?;
                true
            }
            Call::Stream => {
                self.call_stream_fn(offset, addr, args, out)?;
                false
            }
            Call::Generator => {
                self.call_generator_fn(offset, addr, args, out)?;
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
        rhs: InstAddress,
    ) -> VmResult<()> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs, rhs) {
            TargetValue::Value(lhs, rhs) => {
                match (
                    &mut *vm_try!(lhs.borrow_kind_mut()),
                    &*vm_try!(rhs.borrow_kind_ref()),
                ) {
                    (ValueKind::Integer(lhs), ValueKind::Integer(rhs)) => {
                        let out = vm_try!(integer_op(*lhs, *rhs).ok_or_else(error));
                        *lhs = out;
                        return VmResult::Ok(());
                    }
                    (ValueKind::Float(lhs), ValueKind::Float(rhs)) => {
                        let out = float_op(*lhs, *rhs);
                        *lhs = out;
                        return VmResult::Ok(());
                    }
                    _ => {}
                }

                TargetFallback::Value(lhs.clone(), rhs)
            }
            TargetValue::Fallback(fallback) => fallback,
        };

        self.target_fallback_assign(fallback, protocol)
    }

    /// Execute a fallback operation.
    fn target_fallback_assign(
        &mut self,
        fallback: TargetFallback<'_>,
        protocol: Protocol,
    ) -> VmResult<()> {
        match fallback {
            TargetFallback::Value(lhs, rhs) => {
                if let CallResult::Unsupported(lhs) =
                    vm_try!(self.call_instance_fn(lhs, protocol, (&rhs,), Output::discard()))
                {
                    return err(VmErrorKind::UnsupportedBinaryOperation {
                        op: protocol.name,
                        lhs: vm_try!(lhs.type_info()),
                        rhs: vm_try!(rhs.type_info()),
                    });
                };

                VmResult::Ok(())
            }
            TargetFallback::Field(lhs, hash, rhs) => {
                if let CallResult::Unsupported(lhs) = vm_try!(self.call_field_fn(
                    protocol,
                    lhs.clone(),
                    hash,
                    (rhs,),
                    Output::discard()
                )) {
                    return err(VmErrorKind::UnsupportedObjectSlotIndexGet {
                        target: vm_try!(lhs.type_info()),
                    });
                }

                VmResult::Ok(())
            }
            TargetFallback::Index(lhs, index, rhs) => {
                if let CallResult::Unsupported(lhs) = vm_try!(self.call_index_fn(
                    protocol,
                    lhs.clone(),
                    index,
                    (&rhs,),
                    Output::discard()
                )) {
                    return err(VmErrorKind::UnsupportedTupleIndexGet {
                        target: vm_try!(lhs.type_info()),
                        index,
                    });
                }

                VmResult::Ok(())
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
        out: Output,
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.at(rhs)).clone();
        let lhs = vm_try!(self.stack.at(lhs)).clone();

        match (
            &*vm_try!(lhs.borrow_kind_ref()),
            &*vm_try!(rhs.borrow_kind_ref()),
        ) {
            (ValueKind::Integer(lhs), ValueKind::Integer(rhs)) => {
                let value = vm_try!(integer_op(*lhs, *rhs).ok_or_else(error));
                vm_try!(out.store(&mut self.stack, value));
                return VmResult::Ok(());
            }
            (ValueKind::Float(lhs), ValueKind::Float(rhs)) => {
                vm_try!(out.store(&mut self.stack, float_op(*lhs, *rhs)));
                return VmResult::Ok(());
            }
            _ => {}
        };

        let lhs = lhs.clone();
        let rhs = rhs.clone();

        if let CallResult::Unsupported(lhs) =
            vm_try!(self.call_instance_fn(lhs, protocol, (&rhs,), out))
        {
            return err(VmErrorKind::UnsupportedBinaryOperation {
                op: protocol.name,
                lhs: vm_try!(lhs.type_info()),
                rhs: vm_try!(rhs.type_info()),
            });
        }

        VmResult::Ok(())
    }

    /// Internal impl of a numeric operation.
    fn internal_infallible_bitwise_bool(
        &mut self,
        protocol: Protocol,
        integer_op: fn(i64, i64) -> i64,
        byte_op: fn(u8, u8) -> u8,
        bool_op: fn(bool, bool) -> bool,
        lhs: InstAddress,
        rhs: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.at(rhs)).clone();
        let lhs = vm_try!(self.stack.at(lhs)).clone();

        match (
            &*vm_try!(lhs.borrow_kind_ref()),
            &*vm_try!(rhs.borrow_kind_ref()),
        ) {
            (ValueKind::Integer(lhs), ValueKind::Integer(rhs)) => {
                vm_try!(out.store(&mut self.stack, integer_op(*lhs, *rhs)));
                return VmResult::Ok(());
            }
            (ValueKind::Byte(lhs), ValueKind::Byte(rhs)) => {
                vm_try!(out.store(&mut self.stack, byte_op(*lhs, *rhs)));
                return VmResult::Ok(());
            }
            (ValueKind::Bool(lhs), ValueKind::Bool(rhs)) => {
                vm_try!(out.store(&mut self.stack, bool_op(*lhs, *rhs)));
                return VmResult::Ok(());
            }
            _ => {}
        };

        if let CallResult::Unsupported(lhs) =
            vm_try!(self.call_instance_fn(lhs, protocol, (&rhs,), out))
        {
            return err(VmErrorKind::UnsupportedBinaryOperation {
                op: protocol.name,
                lhs: vm_try!(lhs.type_info()),
                rhs: vm_try!(rhs.type_info()),
            });
        }

        VmResult::Ok(())
    }

    fn internal_infallible_bitwise_assign(
        &mut self,
        target: InstTarget,
        protocol: Protocol,
        integer_op: fn(&mut i64, i64),
        byte_op: fn(&mut u8, u8),
        bool_op: fn(&mut bool, bool),
        rhs: InstAddress,
    ) -> VmResult<()> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs, rhs) {
            TargetValue::Value(lhs, rhs) => {
                match (
                    &mut *vm_try!(lhs.borrow_kind_mut()),
                    &*vm_try!(rhs.borrow_kind_ref()),
                ) {
                    (ValueKind::Integer(lhs), ValueKind::Integer(rhs)) => {
                        integer_op(lhs, *rhs);
                        return VmResult::Ok(());
                    }
                    (ValueKind::Byte(lhs), ValueKind::Byte(rhs)) => {
                        byte_op(lhs, *rhs);
                        return VmResult::Ok(());
                    }
                    (ValueKind::Bool(lhs), ValueKind::Bool(rhs)) => {
                        bool_op(lhs, *rhs);
                        return VmResult::Ok(());
                    }
                    _ => {}
                }

                TargetFallback::Value(lhs.clone(), rhs)
            }
            TargetValue::Fallback(fallback) => fallback,
        };

        self.target_fallback_assign(fallback, protocol)
    }

    fn internal_bitwise(
        &mut self,
        protocol: Protocol,
        error: fn() -> VmErrorKind,
        integer_op: fn(i64, i64) -> Option<i64>,
        byte_op: fn(u8, i64) -> Option<u8>,
        lhs: InstAddress,
        rhs: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.at(rhs)).clone();
        let lhs = vm_try!(self.stack.at(lhs)).clone();

        match (
            &*vm_try!(lhs.borrow_kind_ref()),
            &*vm_try!(rhs.borrow_kind_ref()),
        ) {
            (ValueKind::Integer(lhs), ValueKind::Integer(rhs)) => {
                let integer = vm_try!(integer_op(*lhs, *rhs).ok_or_else(error));
                vm_try!(out.store(&mut self.stack, integer));
                return VmResult::Ok(());
            }
            (ValueKind::Byte(lhs), ValueKind::Integer(rhs)) => {
                let byte = vm_try!(byte_op(*lhs, *rhs).ok_or_else(error));
                vm_try!(out.store(&mut self.stack, byte));
                return VmResult::Ok(());
            }
            _ => {}
        }

        if let CallResult::Unsupported(lhs) =
            vm_try!(self.call_instance_fn(lhs, protocol, (&rhs,), out))
        {
            return err(VmErrorKind::UnsupportedBinaryOperation {
                op: protocol.name,
                lhs: vm_try!(lhs.type_info()),
                rhs: vm_try!(rhs.type_info()),
            });
        }

        VmResult::Ok(())
    }

    fn internal_bitwise_assign(
        &mut self,
        target: InstTarget,
        protocol: Protocol,
        error: fn() -> VmErrorKind,
        integer_op: fn(i64, i64) -> Option<i64>,
        byte_op: fn(u8, i64) -> Option<u8>,
        rhs: InstAddress,
    ) -> VmResult<()> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs, rhs) {
            TargetValue::Value(lhs, rhs) => {
                match (
                    &mut *vm_try!(lhs.borrow_kind_mut()),
                    &*vm_try!(rhs.borrow_kind_ref()),
                ) {
                    (ValueKind::Integer(lhs), ValueKind::Integer(rhs)) => {
                        let out = vm_try!(integer_op(*lhs, *rhs).ok_or_else(error));
                        *lhs = out;
                        return VmResult::Ok(());
                    }
                    (ValueKind::Byte(lhs), ValueKind::Integer(rhs)) => {
                        let out = vm_try!(byte_op(*lhs, *rhs).ok_or_else(error));
                        *lhs = out;
                        return VmResult::Ok(());
                    }
                    _ => {}
                }

                TargetFallback::Value(lhs.clone(), rhs)
            }
            TargetValue::Fallback(fallback) => fallback,
        };

        self.target_fallback_assign(fallback, protocol)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_await(&mut self, addr: InstAddress) -> VmResult<Future> {
        vm_try!(self.stack.at(addr)).clone().into_future()
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_select(
        &mut self,
        addr: InstAddress,
        len: usize,
        branch: Output,
    ) -> VmResult<Option<Select>> {
        let futures = futures_util::stream::FuturesUnordered::new();

        for (branch, value) in vm_try!(self.stack.slice_at(addr, len)).iter().enumerate() {
            let future = vm_try!(value.clone().into_future_mut());

            if !future.is_completed() {
                futures.push(SelectFuture::new(branch as i64, future));
            }
        }

        // NB: nothing to poll.
        if futures.is_empty() {
            vm_try!(branch.store(&mut self.stack, ()));
            return VmResult::Ok(None);
        }

        VmResult::Ok(Some(Select::new(futures)))
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_push(&mut self, value: InstValue, out: Output) -> VmResult<()> {
        vm_try!(out.store(&mut self.stack, vm_try!(value.into_value())));
        VmResult::Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_copy(&mut self, addr: InstAddress, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr)).clone();
        vm_try!(out.store(&mut self.stack, value));
        VmResult::Ok(())
    }

    /// Move a value from a position relative to the top of the stack, to the
    /// top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_move(&mut self, addr: InstAddress, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr)).clone();
        let value = vm_try!(value.move_());
        vm_try!(out.store(&mut self.stack, value));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_drop(&mut self, addr: InstAddress) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr)).clone();
        vm_try!(value.drop());
        VmResult::Ok(())
    }

    /// Swap two values on the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_swap(&mut self, a: InstAddress, b: InstAddress) -> VmResult<()> {
        vm_try!(self.stack.swap(a, b));
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
    fn op_jump_if(&mut self, cond: InstAddress, jump: usize) -> VmResult<()> {
        if vm_try!(vm_try!(self.stack.at(cond)).as_bool()) {
            self.ip = vm_try!(self.unit.translate(jump));
        }

        VmResult::Ok(())
    }

    /// pop-and-jump-if-not instruction.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_jump_if_not(&mut self, cond: InstAddress, jump: usize) -> VmResult<()> {
        if !vm_try!(vm_try!(self.stack.at(cond)).as_bool()) {
            self.ip = vm_try!(self.unit.translate(jump));
        }

        VmResult::Ok(())
    }

    /// Perform a branch-conditional jump operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_jump_if_branch(&mut self, branch: InstAddress, value: i64, jump: usize) -> VmResult<()> {
        let branch = vm_try!(self.stack.at(branch));

        if matches!(*vm_try!(branch.borrow_kind_ref()), ValueKind::Integer(branch) if branch == value)
        {
            self.ip = vm_try!(self.unit.translate(jump));
        }

        VmResult::Ok(())
    }

    /// Construct a new vec.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_vec(&mut self, addr: InstAddress, count: usize, out: Output) -> VmResult<()> {
        let vec = vm_try!(self.stack.slice_at(addr, count));
        let vec = vm_try!(vec.iter().cloned().try_collect::<alloc::Vec<Value>>());
        vm_try!(out.store(&mut self.stack, || ValueKind::Vec(Vec::from(vec))));
        VmResult::Ok(())
    }

    /// Construct a new tuple.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple(&mut self, addr: InstAddress, count: usize, out: Output) -> VmResult<()> {
        let tuple = vm_try!(self.stack.slice_at(addr, count));
        let tuple = vm_try!(tuple.iter().cloned().try_collect::<alloc::Vec<Value>>());

        vm_try!(
            out.store(&mut self.stack, || VmResult::Ok(ValueKind::Tuple(vm_try!(
                OwnedTuple::try_from(tuple)
            ))))
        );

        VmResult::Ok(())
    }

    /// Construct a new tuple with a fixed number of arguments.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_n(&mut self, args: &[InstAddress], out: Output) -> VmResult<()> {
        let mut tuple = vm_try!(alloc::Vec::<Value>::try_with_capacity(args.len()));

        for &arg in args {
            let value = vm_try!(self.stack.at(arg)).clone();
            vm_try!(tuple.try_push(value));
        }

        vm_try!(
            out.store(&mut self.stack, || VmResult::Ok(ValueKind::Tuple(vm_try!(
                OwnedTuple::try_from(tuple)
            ))))
        );

        VmResult::Ok(())
    }

    /// Push the tuple that is on top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_environment(&mut self, addr: InstAddress, count: usize, out: Output) -> VmResult<()> {
        let tuple = vm_try!(self.stack.at(addr)).clone();
        let tuple = vm_try!(tuple.borrow_tuple_ref());

        if tuple.len() != count {
            return err(VmErrorKind::BadEnvironmentCount {
                expected: count,
                actual: tuple.len(),
            });
        }

        if let Some(addr) = out.as_addr() {
            let out = vm_try!(self.stack.slice_at_mut(addr, count));

            for (value, out) in tuple.iter().zip(out.iter_mut()) {
                *out = value.clone();
            }
        }

        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_allocate(&mut self, size: usize) -> VmResult<()> {
        vm_try!(self.stack.resize(size));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_not(&mut self, operand: InstAddress, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(operand));

        let value = match *vm_try!(value.borrow_kind_ref()) {
            ValueKind::Bool(value) => vm_try!(Value::try_from(!value)),
            ValueKind::Integer(value) => vm_try!(Value::try_from(!value)),
            ValueKind::Byte(value) => vm_try!(Value::try_from(!value)),
            ref other => {
                let operand = other.type_info();
                return err(VmErrorKind::UnsupportedUnaryOperation { op: "!", operand });
            }
        };

        vm_try!(out.store(&mut self.stack, value));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_neg(&mut self, addr: InstAddress, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr));

        let value = match *vm_try!(value.borrow_kind_ref()) {
            ValueKind::Float(value) => vm_try!(Value::try_from(-value)),
            ValueKind::Integer(value) => vm_try!(Value::try_from(-value)),
            ref other => {
                let operand = other.type_info();
                return err(VmErrorKind::UnsupportedUnaryOperation { op: "-", operand });
            }
        };

        vm_try!(out.store(&mut self.stack, value));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_op(
        &mut self,
        op: InstOp,
        lhs: InstAddress,
        rhs: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        match op {
            InstOp::Add => {
                vm_try!(self.internal_num(
                    Protocol::ADD,
                    || VmErrorKind::Overflow,
                    i64::checked_add,
                    ops::Add::add,
                    lhs,
                    rhs,
                    out,
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
                    out,
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
                    out,
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
                    out,
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
                    out,
                ));
            }
            InstOp::BitAnd => {
                use ops::BitAnd as _;
                vm_try!(self.internal_infallible_bitwise_bool(
                    Protocol::BIT_AND,
                    i64::bitand,
                    u8::bitand,
                    bool::bitand,
                    lhs,
                    rhs,
                    out,
                ));
            }
            InstOp::BitXor => {
                use ops::BitXor as _;
                vm_try!(self.internal_infallible_bitwise_bool(
                    Protocol::BIT_XOR,
                    i64::bitxor,
                    u8::bitxor,
                    bool::bitxor,
                    lhs,
                    rhs,
                    out,
                ));
            }
            InstOp::BitOr => {
                use ops::BitOr as _;
                vm_try!(self.internal_infallible_bitwise_bool(
                    Protocol::BIT_OR,
                    i64::bitor,
                    u8::bitor,
                    bool::bitor,
                    lhs,
                    rhs,
                    out,
                ));
            }
            InstOp::Shl => {
                vm_try!(self.internal_bitwise(
                    Protocol::SHL,
                    || VmErrorKind::Overflow,
                    |a, b| a.checked_shl(u32::try_from(b).ok()?),
                    |a, b| a.checked_shl(u32::try_from(b).ok()?),
                    lhs,
                    rhs,
                    out,
                ));
            }
            InstOp::Shr => {
                vm_try!(self.internal_bitwise(
                    Protocol::SHR,
                    || VmErrorKind::Underflow,
                    |a, b| a.checked_shr(u32::try_from(b).ok()?),
                    |a, b| a.checked_shr(u32::try_from(b).ok()?),
                    lhs,
                    rhs,
                    out
                ));
            }
            InstOp::Gt => {
                vm_try!(self.internal_boolean_ops(
                    |o| matches!(o, Ordering::Greater),
                    lhs,
                    rhs,
                    out
                ));
            }
            InstOp::Gte => {
                vm_try!(self.internal_boolean_ops(
                    |o| matches!(o, Ordering::Greater | Ordering::Equal),
                    lhs,
                    rhs,
                    out
                ));
            }
            InstOp::Lt => {
                vm_try!(self.internal_boolean_ops(|o| matches!(o, Ordering::Less), lhs, rhs, out));
            }
            InstOp::Lte => {
                vm_try!(self.internal_boolean_ops(
                    |o| matches!(o, Ordering::Less | Ordering::Equal),
                    lhs,
                    rhs,
                    out
                ));
            }
            InstOp::Eq => {
                let rhs = vm_try!(self.stack.at(rhs)).clone();
                let lhs = vm_try!(self.stack.at(lhs)).clone();
                let test = vm_try!(Value::partial_eq_with(&lhs, &rhs, self));
                vm_try!(out.store(&mut self.stack, test));
            }
            InstOp::Neq => {
                let rhs = vm_try!(self.stack.at(rhs)).clone();
                let lhs = vm_try!(self.stack.at(lhs)).clone();
                let test = vm_try!(Value::partial_eq_with(&lhs, &rhs, self));
                vm_try!(out.store(&mut self.stack, !test));
            }
            InstOp::And => {
                vm_try!(self.internal_boolean_op(|a, b| a && b, "&&", lhs, rhs, out));
            }
            InstOp::Or => {
                vm_try!(self.internal_boolean_op(|a, b| a || b, "||", lhs, rhs, out));
            }
            InstOp::As => {
                let value = vm_try!(self.as_op(lhs, rhs));
                vm_try!(out.store(&mut self.stack, value));
            }
            InstOp::Is => {
                let is_instance = vm_try!(self.test_is_instance(lhs, rhs));
                vm_try!(out.store(&mut self.stack, is_instance));
            }
            InstOp::IsNot => {
                let is_instance = vm_try!(self.test_is_instance(lhs, rhs));
                vm_try!(out.store(&mut self.stack, !is_instance));
            }
        }

        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_assign(
        &mut self,
        target: InstTarget,
        op: InstAssignOp,
        value: InstAddress,
    ) -> VmResult<()> {
        match op {
            InstAssignOp::Add => {
                vm_try!(self.internal_num_assign(
                    target,
                    Protocol::ADD_ASSIGN,
                    || VmErrorKind::Overflow,
                    i64::checked_add,
                    ops::Add::add,
                    value,
                ));
            }
            InstAssignOp::Sub => {
                vm_try!(self.internal_num_assign(
                    target,
                    Protocol::SUB_ASSIGN,
                    || VmErrorKind::Underflow,
                    i64::checked_sub,
                    ops::Sub::sub,
                    value,
                ));
            }
            InstAssignOp::Mul => {
                vm_try!(self.internal_num_assign(
                    target,
                    Protocol::MUL_ASSIGN,
                    || VmErrorKind::Overflow,
                    i64::checked_mul,
                    ops::Mul::mul,
                    value,
                ));
            }
            InstAssignOp::Div => {
                vm_try!(self.internal_num_assign(
                    target,
                    Protocol::DIV_ASSIGN,
                    || VmErrorKind::DivideByZero,
                    i64::checked_div,
                    ops::Div::div,
                    value,
                ));
            }
            InstAssignOp::Rem => {
                vm_try!(self.internal_num_assign(
                    target,
                    Protocol::REM_ASSIGN,
                    || VmErrorKind::DivideByZero,
                    i64::checked_rem,
                    ops::Rem::rem,
                    value,
                ));
            }
            InstAssignOp::BitAnd => {
                vm_try!(self.internal_infallible_bitwise_assign(
                    target,
                    Protocol::BIT_AND_ASSIGN,
                    ops::BitAndAssign::bitand_assign,
                    ops::BitAndAssign::bitand_assign,
                    ops::BitAndAssign::bitand_assign,
                    value,
                ));
            }
            InstAssignOp::BitXor => {
                vm_try!(self.internal_infallible_bitwise_assign(
                    target,
                    Protocol::BIT_XOR_ASSIGN,
                    ops::BitXorAssign::bitxor_assign,
                    ops::BitXorAssign::bitxor_assign,
                    ops::BitXorAssign::bitxor_assign,
                    value,
                ));
            }
            InstAssignOp::BitOr => {
                vm_try!(self.internal_infallible_bitwise_assign(
                    target,
                    Protocol::BIT_OR_ASSIGN,
                    ops::BitOrAssign::bitor_assign,
                    ops::BitOrAssign::bitor_assign,
                    ops::BitOrAssign::bitor_assign,
                    value,
                ));
            }
            InstAssignOp::Shl => {
                vm_try!(self.internal_bitwise_assign(
                    target,
                    Protocol::SHL_ASSIGN,
                    || VmErrorKind::Overflow,
                    |a, b| a.checked_shl(u32::try_from(b).ok()?),
                    |a, b| a.checked_shl(u32::try_from(b).ok()?),
                    value,
                ));
            }
            InstAssignOp::Shr => {
                vm_try!(self.internal_bitwise_assign(
                    target,
                    Protocol::SHR_ASSIGN,
                    || VmErrorKind::Underflow,
                    |a, b| a.checked_shr(u32::try_from(b).ok()?),
                    |a, b| a.checked_shr(u32::try_from(b).ok()?),
                    value,
                ));
            }
        }

        VmResult::Ok(())
    }

    /// Perform an index set operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_index_set(
        &mut self,
        target: InstAddress,
        index: InstAddress,
        value: InstAddress,
    ) -> VmResult<()> {
        let target = vm_try!(self.stack.at(target));
        let index = vm_try!(self.stack.at(index));
        let value = vm_try!(self.stack.at(value));

        'out: {
            let kind = vm_try!(index.borrow_kind_ref());

            let field = match *kind {
                ValueKind::String(ref string) => string.as_str(),
                _ => break 'out,
            };

            match &mut *vm_try!(target.borrow_kind_mut()) {
                ValueKind::Object(object) => {
                    vm_try!(object.insert(vm_try!(field.try_to_owned()), value.clone()));
                    return VmResult::Ok(());
                }
                ValueKind::Struct(typed_object) => {
                    if let Some(v) = typed_object.get_mut(field) {
                        *v = value.clone();
                        return VmResult::Ok(());
                    }

                    return err(VmErrorKind::MissingField {
                        target: typed_object.type_info(),
                        field: vm_try!(field.try_to_owned()),
                    });
                }
                ValueKind::Variant(variant) => {
                    if let VariantData::Struct(st) = variant.data_mut() {
                        if let Some(v) = st.get_mut(field) {
                            *v = value.clone();
                            return VmResult::Ok(());
                        }
                    }

                    return err(VmErrorKind::MissingField {
                        target: variant.type_info(),
                        field: vm_try!(field.try_to_owned()),
                    });
                }
                _ => {}
            }
        }

        let target = target.clone();
        let index = index.clone();
        let value = value.clone();

        if let CallResult::Unsupported(target) = vm_try!(self.call_instance_fn(
            target,
            Protocol::INDEX_SET,
            (&index, &value),
            Output::discard()
        )) {
            return err(VmErrorKind::UnsupportedIndexSet {
                target: vm_try!(target.type_info()),
                index: vm_try!(index.type_info()),
                value: vm_try!(value.type_info()),
            });
        }

        VmResult::Ok(())
    }

    #[inline]
    #[tracing::instrument(skip(self))]
    fn op_return_internal(&mut self, return_value: Value) -> VmResult<Option<Output>> {
        let (exit, out) = vm_try!(self.pop_call_frame());
        vm_try!(out.store(&mut self.stack, return_value));
        VmResult::Ok(exit.then_some(out))
    }

    fn lookup_function_by_hash(&self, hash: Hash) -> Result<Function, VmErrorKind> {
        Ok(match self.unit.function(hash) {
            Some(info) => match info {
                UnitFn::Offset {
                    offset, call, args, ..
                } => Function::from_vm_offset(
                    self.context.clone(),
                    self.unit.clone(),
                    offset,
                    call,
                    args,
                    hash,
                ),
                UnitFn::EmptyStruct { hash } => {
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
    fn op_return(&mut self, addr: InstAddress) -> VmResult<Option<Output>> {
        let return_value = vm_try!(self.stack.at(addr)).clone();
        self.op_return_internal(return_value)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    #[tracing::instrument(skip(self))]
    fn op_return_unit(&mut self) -> VmResult<bool> {
        let (exit, out) = vm_try!(self.pop_call_frame());
        vm_try!(out.store(&mut self.stack, ()));
        VmResult::Ok(exit)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_load_instance_fn(&mut self, addr: InstAddress, hash: Hash, out: Output) -> VmResult<()> {
        let instance = vm_try!(self.stack.at(addr));
        let ty = vm_try!(instance.type_hash());
        let hash = Hash::associated_function(ty, hash);
        vm_try!(out.store(&mut self.stack, || ValueKind::Type(Type::new(hash))));
        VmResult::Ok(())
    }

    /// Perform an index get operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_index_get(
        &mut self,
        target: InstAddress,
        index: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        let value = 'store: {
            let index = vm_try!(self.stack.at(index));
            let target = vm_try!(self.stack.at(target));

            match &*vm_try!(index.borrow_kind_ref()) {
                ValueKind::String(index) => {
                    if let Some(value) =
                        vm_try!(Self::try_object_like_index_get(target, index.as_str()))
                    {
                        break 'store value;
                    }
                }
                ValueKind::Integer(index) => {
                    let Ok(index) = usize::try_from(*index) else {
                        return err(VmErrorKind::MissingIndexInteger {
                            target: vm_try!(target.type_info()),
                            index: VmIntegerRepr::from(*index),
                        });
                    };

                    if let Some(value) = vm_try!(Self::try_tuple_like_index_get(target, index)) {
                        break 'store value;
                    }
                }
                _ => (),
            }

            let target = target.clone();
            let index = index.clone();

            if let CallResult::Unsupported(target) =
                vm_try!(self.call_instance_fn(target, Protocol::INDEX_GET, (&index,), out))
            {
                return err(VmErrorKind::UnsupportedIndexGet {
                    target: vm_try!(target.type_info()),
                    index: vm_try!(index.type_info()),
                });
            }

            return VmResult::Ok(());
        };

        vm_try!(out.store(&mut self.stack, value));
        VmResult::Ok(())
    }

    /// Perform an index get operation specialized for tuples.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_index_set(
        &mut self,
        target: InstAddress,
        index: usize,
        value: InstAddress,
    ) -> VmResult<()> {
        let target = vm_try!(self.stack.at(target)).clone();
        let value = vm_try!(self.stack.at(value)).clone();

        if vm_try!(Self::try_tuple_like_index_set(&target, index, value)) {
            return VmResult::Ok(());
        }

        err(VmErrorKind::UnsupportedTupleIndexSet {
            target: vm_try!(target.type_info()),
        })
    }

    /// Perform an index get operation specialized for tuples.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_index_get_at(
        &mut self,
        addr: InstAddress,
        index: usize,
        out: Output,
    ) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr));

        if let Some(value) = vm_try!(Self::try_tuple_like_index_get(value, index)) {
            vm_try!(out.store(&mut self.stack, value));
            return VmResult::Ok(());
        }

        let value = value.clone();

        if let CallResult::Unsupported(value) =
            vm_try!(self.call_index_fn(Protocol::GET, value, index, (), out))
        {
            return err(VmErrorKind::UnsupportedTupleIndexGet {
                target: vm_try!(value.type_info()),
                index,
            });
        }

        VmResult::Ok(())
    }

    /// Perform a specialized index set operation on an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object_index_set(
        &mut self,
        target: InstAddress,
        slot: usize,
        value: InstAddress,
    ) -> VmResult<()> {
        let target = vm_try!(self.stack.at(target)).clone();
        let value = vm_try!(self.stack.at(value)).clone();

        if let CallResult::Unsupported(target) =
            vm_try!(self.try_object_slot_index_set(target, slot, value))
        {
            return err(VmErrorKind::UnsupportedObjectSlotIndexSet {
                target: vm_try!(target.type_info()),
            });
        }

        VmResult::Ok(())
    }

    /// Perform a specialized index get operation on an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object_index_get_at(
        &mut self,
        addr: InstAddress,
        slot: usize,
        out: Output,
    ) -> VmResult<()> {
        let target = vm_try!(self.stack.at(addr)).clone();

        match vm_try!(self.try_object_slot_index_get(target, slot, out)) {
            CallResult::Ok(()) => VmResult::Ok(()),
            CallResult::Unsupported(target) => err(VmErrorKind::UnsupportedObjectSlotIndexGet {
                target: vm_try!(target.type_info()),
            }),
        }
    }

    /// Operation to allocate an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object(&mut self, addr: InstAddress, slot: usize, out: Output) -> VmResult<()> {
        let keys = vm_try!(self
            .unit
            .lookup_object_keys(slot)
            .ok_or(VmErrorKind::MissingStaticObjectKeys { slot }));

        let mut object = vm_try!(Object::with_capacity(keys.len()));
        let values = vm_try!(self.stack.slice_at(addr, keys.len()));

        for (key, value) in keys.iter().zip(values) {
            let key = vm_try!(String::try_from(key.as_str()));
            vm_try!(object.insert(key, value.clone()));
        }

        vm_try!(out.store(&mut self.stack, ValueKind::Object(object)));
        VmResult::Ok(())
    }

    /// Operation to allocate an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_range(&mut self, addr: InstAddress, range: InstRange, out: Output) -> VmResult<()> {
        let value = match range {
            InstRange::RangeFrom => {
                let [s] = vm_try!(self.stack.array_at(addr));
                vm_try!(Value::try_from(RangeFrom::new(s.clone())))
            }
            InstRange::RangeFull => {
                vm_try!(Value::try_from(RangeFull::new()))
            }
            InstRange::RangeInclusive => {
                let [s, e] = vm_try!(self.stack.array_at(addr));
                vm_try!(Value::try_from(RangeInclusive::new(s.clone(), e.clone())))
            }
            InstRange::RangeToInclusive => {
                let [e] = vm_try!(self.stack.array_at(addr));
                vm_try!(Value::try_from(RangeToInclusive::new(e.clone())))
            }
            InstRange::RangeTo => {
                let [e] = vm_try!(self.stack.array_at(addr));
                vm_try!(Value::try_from(RangeTo::new(e.clone())))
            }
            InstRange::Range => {
                let [s, e] = vm_try!(self.stack.array_at(addr));
                vm_try!(Value::try_from(Range::new(s.clone(), e.clone())))
            }
        };

        vm_try!(out.store(&mut self.stack, value));
        VmResult::Ok(())
    }

    /// Operation to allocate an empty struct.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_empty_struct(&mut self, hash: Hash, out: Output) -> VmResult<()> {
        let rtti = vm_try!(self
            .unit
            .lookup_rtti(hash)
            .ok_or(VmErrorKind::MissingRtti { hash }));

        vm_try!(
            out.store(&mut self.stack, || ValueKind::EmptyStruct(EmptyStruct {
                rtti: rtti.clone()
            }))
        );

        VmResult::Ok(())
    }

    /// Operation to allocate an object struct.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_struct(
        &mut self,
        addr: InstAddress,
        hash: Hash,
        slot: usize,
        out: Output,
    ) -> VmResult<()> {
        let keys = vm_try!(self
            .unit
            .lookup_object_keys(slot)
            .ok_or(VmErrorKind::MissingStaticObjectKeys { slot }));

        let rtti = vm_try!(self
            .unit
            .lookup_rtti(hash)
            .ok_or(VmErrorKind::MissingRtti { hash }));

        let values = vm_try!(self.stack.slice_at(addr, keys.len()));
        let mut data = vm_try!(Object::with_capacity(keys.len()));

        for (key, value) in keys.iter().zip(values) {
            let key = vm_try!(String::try_from(key.as_str()));
            vm_try!(data.insert(key, value.clone()));
        }

        vm_try!(out.store(&mut self.stack, || ValueKind::Struct(Struct {
            rtti: rtti.clone(),
            data,
        })));

        VmResult::Ok(())
    }

    /// Operation to allocate an object variant.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object_variant(
        &mut self,
        addr: InstAddress,
        hash: Hash,
        slot: usize,
        out: Output,
    ) -> VmResult<()> {
        let keys = vm_try!(self
            .unit
            .lookup_object_keys(slot)
            .ok_or(VmErrorKind::MissingStaticObjectKeys { slot }));

        let rtti = vm_try!(self
            .unit
            .lookup_variant_rtti(hash)
            .ok_or(VmErrorKind::MissingVariantRtti { hash }));

        let mut data = vm_try!(Object::with_capacity(keys.len()));
        let values = vm_try!(self.stack.slice_at(addr, keys.len()));

        for (key, value) in keys.iter().zip(values) {
            let key = vm_try!(String::try_from(key.as_str()));
            vm_try!(data.insert(key, value.clone()));
        }

        vm_try!(
            out.store(&mut self.stack, || ValueKind::Variant(Variant::struct_(
                rtti.clone(),
                data
            )))
        );
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_string(&mut self, slot: usize, out: Output) -> VmResult<()> {
        let string = vm_try!(self.unit.lookup_string(slot));
        vm_try!(out.store(&mut self.stack, || String::try_from(string.as_str())));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_bytes(&mut self, slot: usize, out: Output) -> VmResult<()> {
        let bytes = vm_try!(self.unit.lookup_bytes(slot));
        vm_try!(out.store(&mut self.stack, || Bytes::try_from(bytes)));
        VmResult::Ok(())
    }

    /// Optimize operation to perform string concatenation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_string_concat(
        &mut self,
        addr: InstAddress,
        len: usize,
        size_hint: usize,
        out: Output,
    ) -> VmResult<()> {
        let values = vm_try!(self.stack.slice_at(addr, len));
        let values = vm_try!(values.iter().cloned().try_collect::<alloc::Vec<_>>());

        let mut f = vm_try!(Formatter::with_capacity(size_hint));

        for value in values {
            vm_try!(value.string_display_with(&mut f, &mut *self));
        }

        vm_try!(out.store(&mut self.stack, ValueKind::String(f.string)));
        VmResult::Ok(())
    }

    /// Push a format specification onto the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_format(&mut self, addr: InstAddress, spec: FormatSpec, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr)).clone();
        vm_try!(out.store(&mut self.stack, || ValueKind::Format(Format {
            value,
            spec
        })));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_is_unit(&mut self, addr: InstAddress, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr));
        let is_unit = vm_try!(value.is_empty());
        vm_try!(out.store(&mut self.stack, is_unit));
        VmResult::Ok(())
    }

    /// Perform the try operation on the given stack location.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_try(&mut self, addr: InstAddress, out: Output) -> VmResult<Option<Output>> {
        let value = vm_try!(self.stack.at(addr)).clone();

        let result = 'out: {
            match &*vm_try!(value.borrow_kind_ref()) {
                ValueKind::Result(result) => break 'out vm_try!(result::result_try(result)),
                ValueKind::Option(option) => break 'out vm_try!(option::option_try(option)),
                _ => {}
            }

            let addr = self.stack.addr();

            if let CallResult::Unsupported(target) =
                vm_try!(self.call_instance_fn(value, Protocol::TRY, (), addr.output()))
            {
                return err(VmErrorKind::UnsupportedTryOperand {
                    actual: vm_try!(target.type_info()),
                });
            }

            let value = vm_try!(self.stack.at(addr)).clone();
            vm_try!(ControlFlow::from_value(value))
        };

        match result {
            ControlFlow::Continue(value) => {
                vm_try!(out.store(&mut self.stack, value));
                VmResult::Ok(None)
            }
            ControlFlow::Break(error) => VmResult::Ok(vm_try!(self.op_return_internal(error))),
        }
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_byte(&mut self, addr: InstAddress, value: u8, out: Output) -> VmResult<()> {
        let v = vm_try!(self.stack.at(addr));

        let is_match = match *vm_try!(v.borrow_kind_ref()) {
            ValueKind::Byte(actual) => actual == value,
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_character(&mut self, addr: InstAddress, value: char, out: Output) -> VmResult<()> {
        let v = vm_try!(self.stack.at(addr));

        let is_match = match *vm_try!(v.borrow_kind_ref()) {
            ValueKind::Char(actual) => actual == value,
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_integer(&mut self, addr: InstAddress, value: i64, out: Output) -> VmResult<()> {
        let v = vm_try!(self.stack.at(addr));

        let is_match = match *vm_try!(v.borrow_kind_ref()) {
            ValueKind::Integer(actual) => actual == value,
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_bool(&mut self, addr: InstAddress, value: bool, out: Output) -> VmResult<()> {
        let v = vm_try!(self.stack.at(addr));

        let is_match = match *vm_try!(v.borrow_kind_ref()) {
            ValueKind::Bool(actual) => actual == value,
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    /// Test if the top of stack is equal to the string at the given static
    /// string slot.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_string(&mut self, addr: InstAddress, slot: usize, out: Output) -> VmResult<()> {
        let v = vm_try!(self.stack.at(addr));

        let is_match = match *vm_try!(v.borrow_kind_ref()) {
            ValueKind::String(ref actual) => {
                let string = vm_try!(self.unit.lookup_string(slot));
                actual.as_str() == string.as_str()
            }
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    /// Test if the top of stack is equal to the string at the given static
    /// bytes slot.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_bytes(&mut self, addr: InstAddress, slot: usize, out: Output) -> VmResult<()> {
        let v = vm_try!(self.stack.at(addr));

        let is_match = match *vm_try!(v.borrow_kind_ref()) {
            ValueKind::Bytes(ref actual) => {
                let bytes = vm_try!(self.unit.lookup_bytes(slot));
                *actual == *bytes
            }
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_match_sequence(
        &mut self,
        ty: TypeCheck,
        len: usize,
        exact: bool,
        addr: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr));

        let result = vm_try!(self.on_tuple(ty, value, move |tuple| {
            if exact {
                tuple.len() == len
            } else {
                tuple.len() >= len
            }
        }));

        vm_try!(out.store(&mut self.stack, result.unwrap_or_default()));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_match_type(&mut self, hash: Hash, addr: InstAddress, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr));
        let is_match = vm_try!(value.type_hash()) == hash;
        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_match_variant(
        &mut self,
        enum_hash: Hash,
        variant_hash: Hash,
        index: usize,
        addr: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr));

        let is_match = 'out: {
            match &*vm_try!(value.borrow_kind_ref()) {
                ValueKind::Variant(variant) => {
                    break 'out variant.rtti().hash == variant_hash;
                }
                ValueKind::Any(any) => {
                    if any.type_hash() != enum_hash {
                        break 'out false;
                    }
                }
                _ => break 'out false,
            }

            let value = value.clone();

            let CallResult::Ok(()) =
                vm_try!(self.call_instance_fn(value, Protocol::IS_VARIANT, (index,), out))
            else {
                break 'out false;
            };

            return VmResult::Ok(());
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_match_builtin(
        &mut self,
        type_check: TypeCheck,
        addr: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        use crate::runtime::GeneratorState::*;

        let value = vm_try!(self.stack.at(addr));

        let is_match = match (type_check, &*vm_try!(value.borrow_kind_ref())) {
            (TypeCheck::EmptyTuple, ValueKind::EmptyTuple) => true,
            (TypeCheck::Tuple, ValueKind::Tuple(..)) => true,
            (TypeCheck::Vec, ValueKind::Vec(..)) => true,
            (TypeCheck::Result(v), ValueKind::Result(result)) => match (v, result) {
                (0, Ok(..)) => true,
                (1, Err(..)) => true,
                _ => false,
            },
            (TypeCheck::Option(v), ValueKind::Option(option)) => match (v, option) {
                (0, Some(..)) => true,
                (1, None) => true,
                _ => false,
            },
            (TypeCheck::GeneratorState(v), ValueKind::GeneratorState(state)) => match (v, state) {
                (0, Complete(..)) => true,
                (1, Yielded(..)) => true,
                _ => false,
            },
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_match_object(
        &mut self,
        slot: usize,
        exact: bool,
        addr: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        fn test(object: &Object, keys: &[alloc::String], exact: bool) -> bool {
            if exact {
                if object.len() != keys.len() {
                    return false;
                }
            } else if object.len() < keys.len() {
                return false;
            }

            for key in keys {
                if !object.contains_key(key.as_str()) {
                    return false;
                }
            }

            true
        }

        let value = vm_try!(self.stack.at(addr));

        let is_match = match &*vm_try!(value.borrow_kind_ref()) {
            ValueKind::Object(object) => {
                let keys = vm_try!(self
                    .unit
                    .lookup_object_keys(slot)
                    .ok_or(VmErrorKind::MissingStaticObjectKeys { slot }));

                test(object, keys, exact)
            }
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    /// Push the given variant onto the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_variant(&mut self, addr: InstAddress, variant: InstVariant, out: Output) -> VmResult<()> {
        match variant {
            InstVariant::Some => {
                let some = vm_try!(self.stack.at(addr)).clone();
                vm_try!(out.store(&mut self.stack, || Value::try_from(Some(some))));
            }
            InstVariant::None => {
                vm_try!(out.store(&mut self.stack, || Value::try_from(None)));
            }
            InstVariant::Ok => {
                let ok = vm_try!(self.stack.at(addr)).clone();
                vm_try!(out.store(&mut self.stack, || Value::try_from(Ok(ok))));
            }
            InstVariant::Err => {
                let err = vm_try!(self.stack.at(addr)).clone();
                vm_try!(out.store(&mut self.stack, || Value::try_from(Err(err))));
            }
        }

        VmResult::Ok(())
    }

    /// Load a function as a value onto the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_load_fn(&mut self, hash: Hash, out: Output) -> VmResult<()> {
        let function = vm_try!(self.lookup_function_by_hash(hash));
        vm_try!(out.store(&mut self.stack, || ValueKind::Function(function)));
        VmResult::Ok(())
    }

    /// Construct a closure on the top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_closure(
        &mut self,
        hash: Hash,
        addr: InstAddress,
        count: usize,
        out: Output,
    ) -> VmResult<()> {
        let info = vm_try!(self
            .unit
            .function(hash)
            .ok_or(VmErrorKind::MissingFunction { hash }));

        let UnitFn::Offset {
            offset,
            call,
            args,
            captures: Some(captures),
        } = info
        else {
            return err(VmErrorKind::MissingFunction { hash });
        };

        if captures != count {
            return err(VmErrorKind::BadEnvironmentCount {
                expected: captures,
                actual: count,
            });
        }

        let environment = vm_try!(self.stack.slice_at(addr, count));
        let environment = vm_try!(environment
            .iter()
            .cloned()
            .try_collect::<alloc::Vec<Value>>());
        let environment = vm_try!(environment.try_into_boxed_slice());

        let function = Function::from_vm_closure(
            self.context.clone(),
            self.unit.clone(),
            offset,
            call,
            args,
            environment,
            hash,
        );

        vm_try!(out.store(&mut self.stack, || ValueKind::Function(function)));
        VmResult::Ok(())
    }

    /// Implementation of a function call.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_call(&mut self, hash: Hash, addr: InstAddress, args: usize, out: Output) -> VmResult<()> {
        let Some(info) = self.unit.function(hash) else {
            let handler = vm_try!(self
                .context
                .function(hash)
                .ok_or(VmErrorKind::MissingFunction { hash }));

            vm_try!(handler(&mut self.stack, addr, args, out));
            return VmResult::Ok(());
        };

        match info {
            UnitFn::Offset {
                offset,
                call,
                args: expected,
                ..
            } => {
                vm_try!(check_args(args, expected));
                vm_try!(self.call_offset_fn(offset, call, addr, args, out));
            }
            UnitFn::EmptyStruct { hash } => {
                vm_try!(check_args(args, 0));

                let rtti = vm_try!(self
                    .unit
                    .lookup_rtti(hash)
                    .ok_or(VmErrorKind::MissingRtti { hash }));

                vm_try!(out.store(&mut self.stack, || Value::empty_struct(rtti.clone())));
            }
            UnitFn::TupleStruct {
                hash,
                args: expected,
            } => {
                vm_try!(check_args(args, expected));

                let rtti = vm_try!(self
                    .unit
                    .lookup_rtti(hash)
                    .ok_or(VmErrorKind::MissingRtti { hash }));

                let tuple = vm_try!(self.stack.slice_at(addr, args));
                let tuple = vm_try!(tuple.iter().cloned().try_collect());

                vm_try!(out.store(&mut self.stack, || {
                    Value::tuple_struct(rtti.clone(), tuple)
                }));
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

                let tuple = vm_try!(self.stack.slice_at(addr, args));
                let tuple = vm_try!(tuple.iter().cloned().try_collect());

                vm_try!(out.store(&mut self.stack, || Value::tuple_variant(
                    rtti.clone(),
                    tuple
                )));
            }
            UnitFn::UnitVariant { hash } => {
                vm_try!(check_args(args, 0));

                let rtti = vm_try!(self
                    .unit
                    .lookup_variant_rtti(hash)
                    .ok_or(VmErrorKind::MissingVariantRtti { hash }));

                vm_try!(out.store(&mut self.stack, || Value::unit_variant(rtti.clone())));
            }
        }

        VmResult::Ok(())
    }

    /// Call a function at the given offset with the given number of arguments.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_call_offset(
        &mut self,
        offset: usize,
        call: Call,
        addr: InstAddress,
        args: usize,
        out: Output,
    ) -> VmResult<()> {
        vm_try!(self.call_offset_fn(offset, call, addr, args, out));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_call_associated(
        &mut self,
        hash: Hash,
        addr: InstAddress,
        args: usize,
        out: Output,
    ) -> VmResult<()> {
        let args = args + 1;
        let instance = vm_try!(self.stack.at(addr));
        let type_hash = vm_try!(instance.type_hash());
        let hash = Hash::associated_function(type_hash, hash);

        if let Some(UnitFn::Offset {
            offset,
            call,
            args: expected,
            ..
        }) = self.unit.function(hash)
        {
            vm_try!(check_args(args, expected));
            vm_try!(self.call_offset_fn(offset, call, addr, args, out));
            return VmResult::Ok(());
        }

        if let Some(handler) = self.context.function(hash) {
            vm_try!(self.called_function_hook(hash));
            vm_try!(handler(&mut self.stack, addr, args, out));
            return VmResult::Ok(());
        }

        err(VmErrorKind::MissingInstanceFunction {
            instance: vm_try!(instance.type_info()),
            hash,
        })
    }

    #[cfg_attr(feature = "bench", inline(never))]
    #[tracing::instrument(skip(self))]
    fn op_call_fn(
        &mut self,
        function: InstAddress,
        addr: InstAddress,
        args: usize,
        out: Output,
    ) -> VmResult<Option<VmHalt>> {
        let function = vm_try!(self.stack.at(function)).clone();

        let ty = match *vm_try!(function.borrow_kind_ref()) {
            ValueKind::Type(ty) => ty,
            ValueKind::Function(ref function) => {
                return function.call_with_vm(self, addr, args, out);
            }
            ref actual => {
                let actual = actual.type_info();
                return err(VmErrorKind::UnsupportedCallFn { actual });
            }
        };

        vm_try!(self.op_call(ty.into_hash(), addr, args, out));
        VmResult::Ok(None)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_iter_next(&mut self, addr: InstAddress, jump: usize, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr));

        let some = match &*vm_try!(value.borrow_kind_ref()) {
            ValueKind::Option(option) => match option {
                Some(some) => some.clone(),
                None => {
                    self.ip = vm_try!(self.unit.translate(jump));
                    return VmResult::Ok(());
                }
            },
            actual => {
                return err(VmErrorKind::UnsupportedIterNextOperand {
                    actual: actual.type_info(),
                });
            }
        };

        vm_try!(out.store(&mut self.stack, some));
        VmResult::Ok(())
    }

    /// Call the provided closure within the context of this virtual machine.
    ///
    /// This allows for calling protocol function helpers like
    /// [Value::string_display] which requires access to a virtual machine.
    ///
    /// ```no_run
    /// use rune::{Context, Unit};
    /// use rune::runtime::Formatter;
    /// use std::sync::Arc;
    ///
    /// let context = Context::with_default_modules()?;
    /// let context = Arc::new(context.runtime()?);
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
    /// let mut f = Formatter::new();
    ///
    /// // Note: We do an extra unwrap because the return value is
    /// // `fmt::Result`.
    /// vm.with(|| output.string_display(&mut f)).into_result()?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn with<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let _guard = crate::runtime::env::Guard::new(
            NonNull::from(&self.context),
            NonNull::from(&self.unit),
            None,
        );
        f()
    }

    /// Evaluate a single instruction.
    pub(crate) fn run(&mut self, diagnostics: Option<&mut dyn VmDiagnostics>) -> VmResult<VmHalt> {
        let mut vm_diagnostics_obj;

        let diagnostics = match diagnostics {
            Some(diagnostics) => {
                vm_diagnostics_obj = VmDiagnosticsObj::new(diagnostics);
                Some(NonNull::from(&mut vm_diagnostics_obj))
            }
            None => None,
        };

        // NB: set up environment so that native function can access context and
        // unit.
        let _guard = crate::runtime::env::Guard::new(
            NonNull::from(&self.context),
            NonNull::from(&self.unit),
            diagnostics,
        );

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
            self.last_ip_len = inst_len as u8;

            match inst {
                Inst::Allocate { size } => {
                    vm_try!(self.op_allocate(size));
                }
                Inst::Not { addr, out } => {
                    vm_try!(self.op_not(addr, out));
                }
                Inst::Neg { addr, out } => {
                    vm_try!(self.op_neg(addr, out));
                }
                Inst::Closure {
                    hash,
                    addr,
                    count,
                    out,
                } => {
                    vm_try!(self.op_closure(hash, addr, count, out));
                }
                Inst::Call {
                    hash,
                    addr,
                    args,
                    out,
                } => {
                    vm_try!(self.op_call(hash, addr, args, out));
                }
                Inst::CallOffset {
                    offset,
                    call,
                    addr,
                    args,
                    out,
                } => {
                    vm_try!(self.op_call_offset(offset, call, addr, args, out));
                }
                Inst::CallAssociated {
                    hash,
                    addr,
                    args,
                    out,
                } => {
                    vm_try!(self.op_call_associated(hash, addr, args, out));
                }
                Inst::CallFn {
                    function,
                    addr,
                    args,
                    out,
                } => {
                    if let Some(reason) = vm_try!(self.op_call_fn(function, addr, args, out)) {
                        return VmResult::Ok(reason);
                    }
                }
                Inst::LoadInstanceFn { addr, hash, out } => {
                    vm_try!(self.op_load_instance_fn(addr, hash, out));
                }
                Inst::IndexGet { target, index, out } => {
                    vm_try!(self.op_index_get(target, index, out));
                }
                Inst::TupleIndexSet {
                    target,
                    index,
                    value,
                } => {
                    vm_try!(self.op_tuple_index_set(target, index, value));
                }
                Inst::TupleIndexGetAt { addr, index, out } => {
                    vm_try!(self.op_tuple_index_get_at(addr, index, out));
                }
                Inst::ObjectIndexSet {
                    target,
                    slot,
                    value,
                } => {
                    vm_try!(self.op_object_index_set(target, slot, value));
                }
                Inst::ObjectIndexGetAt { addr, slot, out } => {
                    vm_try!(self.op_object_index_get_at(addr, slot, out));
                }
                Inst::IndexSet {
                    target,
                    index,
                    value,
                } => {
                    vm_try!(self.op_index_set(target, index, value));
                }
                Inst::Return { addr } => {
                    if let Some(out) = vm_try!(self.op_return(addr)) {
                        return VmResult::Ok(VmHalt::Exited(out.as_addr()));
                    }
                }
                Inst::ReturnUnit => {
                    if vm_try!(self.op_return_unit()) {
                        return VmResult::Ok(VmHalt::Exited(None));
                    }
                }
                Inst::Await { addr, out } => {
                    let future = vm_try!(self.op_await(addr));
                    return VmResult::Ok(VmHalt::Awaited(Awaited::Future(future, out)));
                }
                Inst::Select {
                    addr,
                    len,
                    branch,
                    value,
                } => {
                    if let Some(select) = vm_try!(self.op_select(addr, len, branch)) {
                        return VmResult::Ok(VmHalt::Awaited(Awaited::Select(
                            select, branch, value,
                        )));
                    }
                }
                Inst::LoadFn { hash, out } => {
                    vm_try!(self.op_load_fn(hash, out));
                }
                Inst::Store { value, out } => {
                    vm_try!(self.op_push(value, out));
                }
                Inst::Copy { addr, out } => {
                    vm_try!(self.op_copy(addr, out));
                }
                Inst::Move { addr, out } => {
                    vm_try!(self.op_move(addr, out));
                }
                Inst::Drop { addr } => {
                    vm_try!(self.op_drop(addr));
                }
                Inst::Swap { a, b } => {
                    vm_try!(self.op_swap(a, b));
                }
                Inst::Jump { jump } => {
                    vm_try!(self.op_jump(jump));
                }
                Inst::JumpIf { cond, jump } => {
                    vm_try!(self.op_jump_if(cond, jump));
                }
                Inst::JumpIfNot { cond, jump } => {
                    vm_try!(self.op_jump_if_not(cond, jump));
                }
                Inst::JumpIfBranch {
                    branch,
                    value,
                    jump,
                } => {
                    vm_try!(self.op_jump_if_branch(branch, value, jump));
                }
                Inst::Vec { addr, count, out } => {
                    vm_try!(self.op_vec(addr, count, out));
                }
                Inst::Tuple { addr, count, out } => {
                    vm_try!(self.op_tuple(addr, count, out));
                }
                Inst::Tuple1 { args, out } => {
                    vm_try!(self.op_tuple_n(&args[..], out));
                }
                Inst::Tuple2 { args, out } => {
                    vm_try!(self.op_tuple_n(&args[..], out));
                }
                Inst::Tuple3 { args, out } => {
                    vm_try!(self.op_tuple_n(&args[..], out));
                }
                Inst::Tuple4 { args, out } => {
                    vm_try!(self.op_tuple_n(&args[..], out));
                }
                Inst::Environment { addr, count, out } => {
                    vm_try!(self.op_environment(addr, count, out));
                }
                Inst::Object { addr, slot, out } => {
                    vm_try!(self.op_object(addr, slot, out));
                }
                Inst::Range { addr, range, out } => {
                    vm_try!(self.op_range(addr, range, out));
                }
                Inst::EmptyStruct { hash, out } => {
                    vm_try!(self.op_empty_struct(hash, out));
                }
                Inst::Struct {
                    addr,
                    hash,
                    slot,
                    out,
                } => {
                    vm_try!(self.op_struct(addr, hash, slot, out));
                }
                Inst::StructVariant {
                    addr,
                    hash,
                    slot,
                    out,
                } => {
                    vm_try!(self.op_object_variant(addr, hash, slot, out));
                }
                Inst::String { slot, out } => {
                    vm_try!(self.op_string(slot, out));
                }
                Inst::Bytes { slot, out } => {
                    vm_try!(self.op_bytes(slot, out));
                }
                Inst::StringConcat {
                    addr,
                    len,
                    size_hint,
                    out,
                } => {
                    vm_try!(self.op_string_concat(addr, len, size_hint, out));
                }
                Inst::Format { addr, spec, out } => {
                    vm_try!(self.op_format(addr, spec, out));
                }
                Inst::IsUnit { addr, out } => {
                    vm_try!(self.op_is_unit(addr, out));
                }
                Inst::Try { addr, out } => {
                    if let Some(out) = vm_try!(self.op_try(addr, out)) {
                        return VmResult::Ok(VmHalt::Exited(out.as_addr()));
                    }
                }
                Inst::EqByte { addr, value, out } => {
                    vm_try!(self.op_eq_byte(addr, value, out));
                }
                Inst::EqChar { addr, value, out } => {
                    vm_try!(self.op_eq_character(addr, value, out));
                }
                Inst::EqInteger { addr, value, out } => {
                    vm_try!(self.op_eq_integer(addr, value, out));
                }
                Inst::EqBool {
                    addr,
                    value: boolean,
                    out,
                } => {
                    vm_try!(self.op_eq_bool(addr, boolean, out));
                }
                Inst::EqString { addr, slot, out } => {
                    vm_try!(self.op_eq_string(addr, slot, out));
                }
                Inst::EqBytes { addr, slot, out } => {
                    vm_try!(self.op_eq_bytes(addr, slot, out));
                }
                Inst::MatchSequence {
                    type_check,
                    len,
                    exact,
                    addr,
                    out,
                } => {
                    vm_try!(self.op_match_sequence(type_check, len, exact, addr, out));
                }
                Inst::MatchType { hash, addr, out } => {
                    vm_try!(self.op_match_type(hash, addr, out));
                }
                Inst::MatchVariant {
                    enum_hash,
                    variant_hash,
                    index,
                    addr,
                    out,
                } => {
                    vm_try!(self.op_match_variant(enum_hash, variant_hash, index, addr, out));
                }
                Inst::MatchBuiltIn {
                    type_check,
                    addr,
                    out,
                } => {
                    vm_try!(self.op_match_builtin(type_check, addr, out));
                }
                Inst::MatchObject {
                    slot,
                    exact,
                    addr,
                    out,
                } => {
                    vm_try!(self.op_match_object(slot, exact, addr, out));
                }
                Inst::Yield { addr, out } => {
                    return VmResult::Ok(VmHalt::Yielded(Some(addr), out));
                }
                Inst::YieldUnit { out } => {
                    return VmResult::Ok(VmHalt::Yielded(None, out));
                }
                Inst::Variant { addr, variant, out } => {
                    vm_try!(self.op_variant(addr, variant, out));
                }
                Inst::Op { op, a, b, out } => {
                    vm_try!(self.op_op(op, a, b, out));
                }
                Inst::Assign { target, op, value } => {
                    vm_try!(self.op_assign(target, op, value));
                }
                Inst::IterNext { addr, jump, out } => {
                    vm_try!(self.op_iter_next(addr, jump, out));
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

impl TryClone for Vm {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            context: self.context.clone(),
            unit: self.unit.clone(),
            ip: self.ip,
            last_ip_len: self.last_ip_len,
            stack: self.stack.try_clone()?,
            call_frames: self.call_frames.try_clone()?,
        })
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
    /// Indicates that the call frame is isolated and should force an exit into
    /// the vm execution context.
    pub isolated: bool,
    /// Keep the value produced from the call frame.
    pub out: Output,
}

impl TryClone for CallFrame {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(*self)
    }
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

    Ok(())
}
