use core::cmp::Ordering;
use core::fmt;
use core::mem::replace;
use core::ops;
use core::ptr::NonNull;
use core::slice;

use ::rust_alloc::sync::Arc;

use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::hash::{Hash, IntoHash, ToTypeHash};
use crate::modules::{option, result};

use super::{
    budget, static_type, Args, Awaited, BorrowMut, BorrowRefRepr, Bytes, Call, ControlFlow,
    DynArgs, DynGuardedArgs, EmptyStruct, Format, FormatSpec, Formatter, FromValue, Function,
    Future, Generator, GuardedArgs, Inline, Inst, InstAddress, InstAssignOp, InstOp, InstRange,
    InstTarget, InstValue, InstVariant, MutRepr, Mutable, Object, Output, OwnedTuple, Pair, Panic,
    Protocol, ProtocolCaller, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
    RangeToInclusive, RefRepr, RuntimeContext, Select, SelectFuture, Stack, Stream, Struct, Type,
    TypeCheck, TypeOf, Unit, UnitFn, UnitStorage, Value, Variant, VariantData, Vec, VmDiagnostics,
    VmDiagnosticsObj, VmError, VmErrorKind, VmExecution, VmHalt, VmIntegerRepr, VmResult,
    VmSendExecution,
};

/// Helper to take a value, replacing the old one with empty.
#[inline(always)]
fn take(value: &mut Value) -> Value {
    replace(value, Value::empty())
}

#[inline(always)]
fn consume(value: &mut Value) {
    *value = Value::empty();
}

/// Indicating the kind of isolation that is present for a frame.
#[derive(Debug, Clone, Copy)]
pub enum Isolated {
    /// The frame is isolated, once pop it will cause the execution to complete.
    Isolated,
    /// No isolation is present, the vm will continue executing.
    None,
}

impl Isolated {
    #[inline]
    pub(crate) fn new(value: bool) -> Self {
        if value {
            Self::Isolated
        } else {
            Self::None
        }
    }

    #[inline]
    pub(crate) fn then_some<T>(self, value: T) -> Option<T> {
        match self {
            Self::Isolated => Some(value),
            Self::None => None,
        }
    }
}

impl fmt::Display for Isolated {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Isolated => write!(f, "isolated"),
            Self::None => write!(f, "none"),
        }
    }
}

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
pub(crate) enum CallResultOnly<T> {
    /// Call successful. Return value is on the stack.
    Ok(T),
    /// Call failed because function was missing so the method is unsupported.
    /// Contains target value.
    Unsupported(Value),
}

/// The result from a dynamic call. Indicates if the attempted operation is
/// supported.
#[derive(Debug)]
pub(crate) enum CallResult<T> {
    /// Call successful. Return value is on the stack.
    Ok(T),
    /// A call frame was pushed onto the virtual machine, which needs to be
    /// advanced to produce the result.
    Frame,
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
    Same(&'a mut Value),
    /// Resolved internal target to mutable value.
    Pair(&'a mut Value, &'a Value),
    /// Fallback to a different kind of operation.
    Fallback(TargetFallback<'b>),
}

macro_rules! target_value {
    ($vm:ident, $target:expr, $guard:ident, $lhs:ident, $rhs:ident) => {{
        match $target {
            InstTarget::Address(addr) => match vm_try!($vm.stack.pair(addr, $rhs)) {
                Pair::Same(value) => TargetValue::Same(value),
                Pair::Pair(lhs, rhs) => TargetValue::Pair(lhs, rhs),
            },
            InstTarget::TupleField(lhs, index) => {
                let rhs = vm_try!($vm.stack.at($rhs));

                $lhs = vm_try!($vm.stack.at(lhs)).clone();

                if let Some(value) = vm_try!(Vm::try_tuple_like_index_get_mut(&$lhs, index)) {
                    $guard = value;
                    TargetValue::Pair(&mut *$guard, rhs)
                } else {
                    TargetValue::Fallback(TargetFallback::Index(&$lhs, index, rhs.clone()))
                }
            }
            InstTarget::Field(lhs, field) => {
                let rhs = vm_try!($vm.stack.at($rhs));

                let field = vm_try!($vm.unit.lookup_string(field));

                $lhs = vm_try!($vm.stack.at(lhs)).clone();

                if let Some(value) = vm_try!(Vm::try_object_like_index_get_mut(&$lhs, field)) {
                    $guard = value;
                    TargetValue::Pair(&mut *$guard, rhs)
                } else {
                    TargetValue::Fallback(TargetFallback::Field(&$lhs, field.hash(), rhs.clone()))
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
    /// use rune::{Context, Unit, Vm};
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
    /// let value = dynamic_max.call::<i64>((10, 20)).into_result()?;
    /// assert_eq!(value, 20);
    ///
    /// // Building an item buffer to lookup an `::std` item.
    /// let item = rune::item!(::std::i64::max);
    /// let max = vm.lookup_function(item)?;
    ///
    /// let value = max.call::<i64>((10, 20)).into_result()?;
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
    #[inline]
    pub(crate) fn call_instance_fn(
        &mut self,
        isolated: Isolated,
        target: Value,
        hash: impl ToTypeHash,
        args: &mut dyn DynArgs,
        out: Output,
    ) -> VmResult<CallResult<()>> {
        let count = args.count().wrapping_add(1);
        let type_hash = vm_try!(target.type_hash());
        let hash = Hash::associated_function(type_hash, hash.to_type_hash());
        self.call_hash_with(isolated, hash, target, args, count, out)
    }

    /// Helper to call a field function.
    #[inline]
    fn call_field_fn(
        &mut self,
        protocol: Protocol,
        target: Value,
        name: impl IntoHash,
        args: &mut dyn DynArgs,
        out: Output,
    ) -> VmResult<CallResult<()>> {
        let count = args.count().wrapping_add(1);
        let hash = Hash::field_function(protocol, vm_try!(target.type_hash()), name);
        self.call_hash_with(Isolated::None, hash, target, args, count, out)
    }

    /// Helper to call an index function.
    #[inline]
    fn call_index_fn(
        &mut self,
        protocol: Protocol,
        target: Value,
        index: usize,
        args: &mut dyn DynArgs,
        out: Output,
    ) -> VmResult<CallResult<()>> {
        let count = args.count().wrapping_add(1);
        let hash = Hash::index_function(protocol, vm_try!(target.type_hash()), Hash::index(index));
        self.call_hash_with(Isolated::None, hash, target, args, count, out)
    }

    fn called_function_hook(&self, hash: Hash) -> VmResult<()> {
        crate::runtime::env::exclusive(|_, _, diagnostics| {
            if let Some(diagnostics) = diagnostics {
                vm_try!(diagnostics.function_used(hash, self.ip()));
            }

            VmResult::Ok(())
        })
    }

    #[inline(never)]
    fn call_hash_with(
        &mut self,
        isolated: Isolated,
        hash: Hash,
        target: Value,
        args: &mut dyn DynArgs,
        count: usize,
        out: Output,
    ) -> VmResult<CallResult<()>> {
        if let Some(handler) = self.context.function(hash) {
            let addr = self.stack.addr();

            vm_try!(self.called_function_hook(hash));
            vm_try!(self.stack.push(target));
            vm_try!(args.push_to_stack(&mut self.stack));

            let result = handler(&mut self.stack, addr, count, out);
            self.stack.truncate(addr);
            vm_try!(result);
            return VmResult::Ok(CallResult::Ok(()));
        }

        if let Some(UnitFn::Offset {
            offset,
            call,
            args: expected,
            ..
        }) = self.unit.function(hash)
        {
            vm_try!(check_args(count, expected));

            let addr = self.stack.addr();
            vm_try!(self.called_function_hook(hash));
            vm_try!(self.stack.push(target));
            vm_try!(args.push_to_stack(&mut self.stack));

            let result = self.call_offset_fn(offset, call, addr, count, isolated, out);

            if vm_try!(result) {
                self.stack.truncate(addr);
                return VmResult::Ok(CallResult::Frame);
            } else {
                return VmResult::Ok(CallResult::Ok(()));
            }
        }

        VmResult::Ok(CallResult::Unsupported(target))
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn internal_boolean_ops(
        &mut self,
        match_ordering: fn(Ordering) -> bool,
        lhs: InstAddress,
        rhs: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.at(rhs));
        let lhs = vm_try!(self.stack.at(lhs));

        let ordering = match (lhs.as_inline_unchecked(), rhs.as_inline_unchecked()) {
            (Some(lhs), Some(rhs)) => vm_try!(lhs.partial_cmp(rhs)),
            _ => {
                let lhs = lhs.clone();
                let rhs = rhs.clone();
                vm_try!(Value::partial_cmp_with(&lhs, &rhs, self))
            }
        };

        vm_try!(out.store(&mut self.stack, || match ordering {
            Some(ordering) => match_ordering(ordering),
            None => false,
        }));

        VmResult::Ok(())
    }

    /// Push a new call frame.
    ///
    /// This will cause the `args` number of elements on the stack to be
    /// associated and accessible to the new call frame.
    #[tracing::instrument(skip(self), fields(call_frames = self.call_frames.len(), top = self.stack.top(), stack = self.stack.len(), self.ip))]
    pub(crate) fn push_call_frame(
        &mut self,
        ip: usize,
        addr: InstAddress,
        args: usize,
        isolated: Isolated,
        out: Output,
    ) -> Result<(), VmErrorKind> {
        tracing::trace!("pushing call frame");

        let top = self.stack.swap_top(addr, args)?;
        let ip = replace(&mut self.ip, ip);

        let frame = CallFrame {
            ip,
            top,
            isolated,
            out,
        };

        self.call_frames.try_push(frame)?;
        Ok(())
    }

    /// Pop a call frame from an internal call, which needs the current stack
    /// pointer to be returned and does not check for context isolation through
    /// [`CallFrame::isolated`].
    #[tracing::instrument(skip(self), fields(call_frames = self.call_frames.len(), top = self.stack.top(), stack = self.stack.len(), self.ip))]
    pub(crate) fn pop_call_frame_from_call(&mut self) -> Result<Option<usize>, VmErrorKind> {
        tracing::trace!("popping call frame from call");

        let Some(frame) = self.call_frames.pop() else {
            return Ok(None);
        };

        tracing::trace!(?frame);
        self.stack.pop_stack_top(frame.top)?;
        Ok(Some(replace(&mut self.ip, frame.ip)))
    }

    /// Pop a call frame and return it.
    #[tracing::instrument(skip(self), fields(call_frames = self.call_frames.len(), top = self.stack.top(), stack = self.stack.len(), self.ip))]
    pub(crate) fn pop_call_frame(&mut self) -> Result<(Isolated, Option<Output>), VmErrorKind> {
        tracing::trace!("popping call frame");

        let Some(frame) = self.call_frames.pop() else {
            self.stack.pop_stack_top(0)?;
            return Ok((Isolated::Isolated, None));
        };

        tracing::trace!(?frame);
        self.stack.pop_stack_top(frame.top)?;
        self.ip = frame.ip;
        Ok((frame.isolated, Some(frame.out)))
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_object_like_index_get(target: &Value, field: &str) -> VmResult<Option<Value>> {
        let BorrowRefRepr::Mutable(target) = vm_try!(target.borrow_ref_repr()) else {
            return VmResult::Ok(None);
        };

        let value = match &*target {
            Mutable::Object(target) => target.get(field),
            Mutable::Struct(target) => target.get(field),
            Mutable::Variant(variant) => match variant.data() {
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

        let result = match vm_try!(target.borrow_ref_repr()) {
            BorrowRefRepr::Inline(target) => match target {
                Inline::Unit => Err(target.type_info()),
                _ => return VmResult::Ok(None),
            },
            BorrowRefRepr::Mutable(target) => match &*target {
                Mutable::Tuple(tuple) => match tuple.get(index) {
                    Some(value) => Ok(value.clone()),
                    None => Err(target.type_info()),
                },
                Mutable::Vec(vec) => match vec.get(index) {
                    Some(value) => Ok(value.clone()),
                    None => Err(target.type_info()),
                },
                Mutable::Result(result) => match (index, result) {
                    (0, Ok(value)) => Ok(value.clone()),
                    (0, Err(value)) => Ok(value.clone()),
                    _ => Err(target.type_info()),
                },
                Mutable::Option(option) => match (index, option) {
                    (0, Some(value)) => Ok(value.clone()),
                    _ => Err(target.type_info()),
                },
                Mutable::GeneratorState(state) => match (index, state) {
                    (0, Yielded(value)) => Ok(value.clone()),
                    (0, Complete(value)) => Ok(value.clone()),
                    _ => Err(target.type_info()),
                },
                Mutable::TupleStruct(tuple_struct) => match tuple_struct.data().get(index) {
                    Some(value) => Ok(value.clone()),
                    None => Err(target.type_info()),
                },
                Mutable::Variant(variant) => match variant.data() {
                    VariantData::Tuple(tuple) => match tuple.get(index) {
                        Some(value) => Ok(value.clone()),
                        None => Err(target.type_info()),
                    },
                    _ => return VmResult::Ok(None),
                },
                _ => return VmResult::Ok(None),
            },
            BorrowRefRepr::Any(..) => {
                return VmResult::Ok(None);
            }
        };

        match result {
            Ok(value) => VmResult::Ok(Some(value)),
            Err(target) => err(VmErrorKind::MissingIndexInteger {
                target,
                index: VmIntegerRepr::from(index),
            }),
        }
    }

    /// Implementation of getting a mutable value out of a tuple-like value.
    fn try_tuple_like_index_get_mut(
        target: &Value,
        index: usize,
    ) -> VmResult<Option<BorrowMut<'_, Value>>> {
        use crate::runtime::GeneratorState::*;

        let mut unsupported = false;

        let result = match vm_try!(target.as_ref_repr()) {
            RefRepr::Mutable(value) => BorrowMut::try_map(vm_try!(value.borrow_mut()), |kind| {
                match kind {
                    Mutable::Tuple(tuple) => return tuple.get_mut(index),
                    Mutable::Vec(vec) => return vec.get_mut(index),
                    Mutable::Result(result) => match (index, result) {
                        (0, Ok(value)) => return Some(value),
                        (0, Err(value)) => return Some(value),
                        _ => return None,
                    },
                    Mutable::Option(option) => match (index, option) {
                        (0, Some(value)) => return Some(value),
                        _ => return None,
                    },
                    Mutable::GeneratorState(state) => match (index, state) {
                        (0, Yielded(value)) => return Some(value),
                        (0, Complete(value)) => return Some(value),
                        _ => return None,
                    },
                    Mutable::TupleStruct(tuple_struct) => return tuple_struct.get_mut(index),
                    Mutable::Variant(Variant {
                        data: VariantData::Tuple(tuple),
                        ..
                    }) => {
                        return tuple.get_mut(index);
                    }
                    _ => {}
                }

                unsupported = true;
                None
            }),
            _ => return VmResult::Ok(None),
        };

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
        match vm_try!(target.as_ref_repr()) {
            RefRepr::Inline(actual) => err(VmErrorKind::MissingField {
                target: actual.type_info(),
                field: vm_try!(field.try_to_owned()),
            }),
            RefRepr::Mutable(target) => {
                let target = vm_try!(target.borrow_mut());

                let mut unsupported = false;

                let result = BorrowMut::try_map(target, |value| {
                    match value {
                        Mutable::Object(target) => {
                            return target.get_mut(field);
                        }
                        Mutable::Struct(target) => {
                            return target.get_mut(field);
                        }
                        Mutable::Variant(Variant {
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
            RefRepr::Any(..) => VmResult::Ok(None),
        }
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_tuple_like_index_set(target: &Value, index: usize, value: &Value) -> VmResult<bool> {
        match vm_try!(target.as_ref_repr()) {
            RefRepr::Inline(target) => match target {
                Inline::Unit => VmResult::Ok(false),
                _ => VmResult::Ok(false),
            },
            RefRepr::Mutable(target) => match &mut *vm_try!(target.borrow_mut()) {
                Mutable::Tuple(tuple) => {
                    if let Some(target) = tuple.get_mut(index) {
                        target.clone_from(value);
                        return VmResult::Ok(true);
                    }

                    VmResult::Ok(false)
                }
                Mutable::Vec(vec) => {
                    if let Some(target) = vec.get_mut(index) {
                        target.clone_from(value);
                        return VmResult::Ok(true);
                    }

                    VmResult::Ok(false)
                }
                Mutable::Result(result) => {
                    let target = match result {
                        Ok(ok) if index == 0 => ok,
                        Err(err) if index == 1 => err,
                        _ => return VmResult::Ok(false),
                    };

                    target.clone_from(value);
                    VmResult::Ok(true)
                }
                Mutable::Option(option) => {
                    let target = match option {
                        Some(some) if index == 0 => some,
                        _ => return VmResult::Ok(false),
                    };

                    target.clone_from(value);
                    VmResult::Ok(true)
                }
                Mutable::TupleStruct(tuple_struct) => {
                    if let Some(target) = tuple_struct.get_mut(index) {
                        target.clone_from(value);
                        return VmResult::Ok(true);
                    }

                    VmResult::Ok(false)
                }
                Mutable::Variant(variant) => {
                    if let VariantData::Tuple(data) = variant.data_mut() {
                        if let Some(target) = data.get_mut(index) {
                            target.clone_from(value);
                            return VmResult::Ok(true);
                        }
                    }

                    VmResult::Ok(false)
                }
                _ => VmResult::Ok(false),
            },
            RefRepr::Any(..) => VmResult::Ok(false),
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

        'fallback: {
            match vm_try!(target.as_ref_repr()) {
                RefRepr::Inline(..) => {
                    return VmResult::Ok(CallResult::Unsupported(target.clone()));
                }
                RefRepr::Mutable(target) => match &mut *vm_try!(target.borrow_mut()) {
                    Mutable::Object(object) => {
                        if let Some(value) = object.get(index.as_str()) {
                            vm_try!(out.store(&mut self.stack, || value.clone()));
                            return VmResult::Ok(CallResult::Ok(()));
                        }
                    }
                    Mutable::Struct(typed_object) => {
                        if let Some(value) = typed_object.get(index.as_str()) {
                            vm_try!(out.store(&mut self.stack, || value.clone()));
                            return VmResult::Ok(CallResult::Ok(()));
                        }
                    }
                    Mutable::Variant(Variant {
                        data: VariantData::Struct(data),
                        ..
                    }) => {
                        if let Some(value) = data.get(index.as_str()) {
                            vm_try!(out.store(&mut self.stack, || value.clone()));
                            return VmResult::Ok(CallResult::Ok(()));
                        }
                    }
                    _ => {
                        break 'fallback;
                    }
                },
                RefRepr::Any(..) => break 'fallback,
            }

            return err(VmErrorKind::ObjectIndexMissing { slot });
        };

        let hash = index.hash();
        let result = vm_try!(self.call_field_fn(Protocol::GET, target, hash, &mut (), out));
        VmResult::Ok(result)
    }

    fn try_object_slot_index_set(target: &Value, field: &str, value: &Value) -> VmResult<bool> {
        match vm_try!(target.as_ref_repr()) {
            RefRepr::Inline(target) => err(VmErrorKind::MissingField {
                target: target.type_info(),
                field: vm_try!(field.try_to_owned()),
            }),
            RefRepr::Mutable(target) => {
                let mut target = vm_try!(target.borrow_mut());

                match &mut *target {
                    Mutable::Object(object) => {
                        let key = vm_try!(field.try_to_owned());
                        vm_try!(object.insert(key, value.clone()));
                        return VmResult::Ok(true);
                    }
                    Mutable::Struct(object) => {
                        if let Some(v) = object.get_mut(field) {
                            v.clone_from(value);
                            return VmResult::Ok(true);
                        }
                    }
                    Mutable::Variant(variant) => {
                        if let VariantData::Struct(data) = variant.data_mut() {
                            if let Some(v) = data.get_mut(field) {
                                v.clone_from(value);
                                return VmResult::Ok(true);
                            }
                        }
                    }
                    _ => {
                        return VmResult::Ok(false);
                    }
                }

                err(VmErrorKind::MissingField {
                    target: target.type_info(),
                    field: vm_try!(field.try_to_owned()),
                })
            }
            RefRepr::Any(..) => VmResult::Ok(false),
        }
    }

    fn on_tuple<F, O>(&self, ty: TypeCheck, value: &Value, f: F) -> VmResult<Option<O>>
    where
        F: FnOnce(&[Value]) -> O,
    {
        use crate::runtime::GeneratorState::*;

        VmResult::Ok(match vm_try!(value.borrow_ref_repr()) {
            BorrowRefRepr::Inline(value) => match (ty, value) {
                (TypeCheck::Unit, Inline::Unit) => Some(f(&[])),
                _ => None,
            },
            BorrowRefRepr::Mutable(value) => match (ty, &*value) {
                (TypeCheck::Tuple, Mutable::Tuple(tuple)) => Some(f(tuple)),
                (TypeCheck::Vec, Mutable::Vec(vec)) => Some(f(vec)),
                (TypeCheck::Result(v), Mutable::Result(result)) => Some(match (v, result) {
                    (0, Ok(ok)) => f(slice::from_ref(ok)),
                    (1, Err(err)) => f(slice::from_ref(err)),
                    _ => return VmResult::Ok(None),
                }),
                (TypeCheck::Option(v), Mutable::Option(option)) => Some(match (v, option) {
                    (0, Some(some)) => f(slice::from_ref(some)),
                    (1, None) => f(&[]),
                    _ => return VmResult::Ok(None),
                }),
                (TypeCheck::GeneratorState(v), Mutable::GeneratorState(state)) => {
                    Some(match (v, state) {
                        (0, Complete(complete)) => f(slice::from_ref(complete)),
                        (1, Yielded(yielded)) => f(slice::from_ref(yielded)),
                        _ => return VmResult::Ok(None),
                    })
                }
                _ => None,
            },
            BorrowRefRepr::Any(..) => None,
        })
    }

    /// Internal implementation of the instance check.
    fn as_op(&mut self, lhs: InstAddress, rhs: InstAddress) -> VmResult<Value> {
        let b = vm_try!(self.stack.at(rhs));
        let a = vm_try!(self.stack.at(lhs));

        let RefRepr::Inline(Inline::Type(ty)) = vm_try!(b.as_ref_repr()) else {
            return err(VmErrorKind::UnsupportedIs {
                value: vm_try!(a.type_info()),
                test_type: vm_try!(b.type_info()),
            });
        };

        macro_rules! convert {
            ($from:ty, $value:expr) => {
                match ty.into_hash() {
                    static_type::FLOAT_HASH => Value::from($value as f64),
                    static_type::BYTE_HASH => Value::from($value as u8),
                    static_type::SIGNED_HASH => Value::from($value as i64),
                    static_type::UNSIGNED_HASH => Value::from($value as u64),
                    ty => {
                        return err(VmErrorKind::UnsupportedAs {
                            value: <$from as TypeOf>::type_info(),
                            type_hash: ty,
                        });
                    }
                }
            };
        }

        let value = match vm_try!(a.as_ref_repr()) {
            RefRepr::Inline(Inline::Signed(a)) => convert!(i64, *a),
            RefRepr::Inline(Inline::Unsigned(a)) => convert!(u64, *a),
            RefRepr::Inline(Inline::Float(a)) => convert!(f64, *a),
            RefRepr::Inline(Inline::Byte(a)) => convert!(u8, *a),
            value => {
                return err(VmErrorKind::UnsupportedAs {
                    value: vm_try!(value.type_info()),
                    type_hash: ty.into_hash(),
                });
            }
        };

        VmResult::Ok(value)
    }

    /// Internal implementation of the instance check.
    fn test_is_instance(&mut self, lhs: InstAddress, rhs: InstAddress) -> VmResult<bool> {
        let b = vm_try!(self.stack.at(rhs));
        let a = vm_try!(self.stack.at(lhs));

        let Some(Inline::Type(ty)) = vm_try!(b.as_inline()) else {
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
        let rhs = vm_try!(self.stack.at(rhs));
        let lhs = vm_try!(self.stack.at(lhs));

        let inline = match (vm_try!(lhs.as_ref_repr()), vm_try!(rhs.as_ref_repr())) {
            (RefRepr::Inline(lhs), RefRepr::Inline(rhs)) => match (lhs, rhs) {
                (Inline::Bool(lhs), Inline::Bool(rhs)) => Inline::Bool(bool_op(*lhs, *rhs)),
                (lhs, rhs) => {
                    return err(VmErrorKind::UnsupportedBinaryOperation {
                        op,
                        lhs: lhs.type_info(),
                        rhs: rhs.type_info(),
                    });
                }
            },
            (lhs, rhs) => {
                return err(VmErrorKind::UnsupportedBinaryOperation {
                    op,
                    lhs: vm_try!(lhs.type_info()),
                    rhs: vm_try!(rhs.type_info()),
                });
            }
        };

        vm_try!(out.store(&mut self.stack, inline));
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
        let values = self.stack.slice_at_mut(addr, args)?;

        if let Some(at) = out.as_addr() {
            let stack = values.iter_mut().map(take).try_collect::<Stack>()?;
            let mut vm = Self::with_stack(self.context.clone(), self.unit.clone(), stack);
            vm.ip = offset;
            *self.stack.at_mut(at)? = Value::try_from(Generator::new(vm))?;
        } else {
            values.iter_mut().for_each(consume);
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
        let values = self.stack.slice_at_mut(addr, args)?;

        if let Some(at) = out.as_addr() {
            let stack = values.iter_mut().map(take).try_collect::<Stack>()?;
            let mut vm = Self::with_stack(self.context.clone(), self.unit.clone(), stack);
            vm.ip = offset;
            *self.stack.at_mut(at)? = Value::try_from(Stream::new(vm))?;
        } else {
            values.iter_mut().for_each(consume);
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
        let values = self.stack.slice_at_mut(addr, args)?;

        if let Some(at) = out.as_addr() {
            let stack = values.iter_mut().map(take).try_collect::<Stack>()?;
            let mut vm = Self::with_stack(self.context.clone(), self.unit.clone(), stack);
            vm.ip = offset;
            let mut execution = vm.into_execution();
            let future = Future::new(async move { execution.async_complete().await })?;
            *self.stack.at_mut(at)? = Value::try_from(future)?;
        } else {
            values.iter_mut().for_each(consume);
        }

        Ok(())
    }

    /// Helper function to call the function at the given offset.
    #[cfg_attr(feature = "bench", inline(never))]
    fn call_offset_fn(
        &mut self,
        offset: usize,
        call: Call,
        addr: InstAddress,
        args: usize,
        isolated: Isolated,
        out: Output,
    ) -> Result<bool, VmErrorKind> {
        let moved = match call {
            Call::Async => {
                self.call_async_fn(offset, addr, args, out)?;
                false
            }
            Call::Immediate => {
                self.push_call_frame(offset, addr, args, isolated, out)?;
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

    #[cfg_attr(feature = "bench", inline(never))]
    fn internal_num_assign(
        &mut self,
        target: InstTarget,
        protocol: Protocol,
        error: fn() -> VmErrorKind,
        signed_op: fn(i64, i64) -> Option<i64>,
        unsigned_op: fn(u64, u64) -> Option<u64>,
        float_op: fn(f64, f64) -> f64,
        rhs: InstAddress,
    ) -> VmResult<()> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs, rhs) {
            TargetValue::Same(value) => {
                match vm_try!(value.as_mut_repr()) {
                    MutRepr::Inline(Inline::Signed(value)) => {
                        let out = vm_try!(signed_op(*value, *value).ok_or_else(error));
                        *value = out;
                        return VmResult::Ok(());
                    }
                    MutRepr::Inline(Inline::Unsigned(value)) => {
                        let out = vm_try!(unsigned_op(*value, *value).ok_or_else(error));
                        *value = out;
                        return VmResult::Ok(());
                    }
                    MutRepr::Inline(Inline::Float(value)) => {
                        let out = float_op(*value, *value);
                        *value = out;
                        return VmResult::Ok(());
                    }
                    MutRepr::Inline(value) => {
                        return err(VmErrorKind::UnsupportedBinaryOperation {
                            op: protocol.name,
                            lhs: value.type_info(),
                            rhs: value.type_info(),
                        });
                    }
                    _ => {}
                }

                TargetFallback::Value(value.clone(), value.clone())
            }
            TargetValue::Pair(lhs, rhs) => {
                match (vm_try!(lhs.as_mut_repr()), vm_try!(rhs.as_ref_repr())) {
                    (MutRepr::Inline(lhs), RefRepr::Inline(rhs)) => match (lhs, rhs) {
                        (Inline::Signed(lhs), Inline::Signed(rhs)) => {
                            let out = vm_try!(signed_op(*lhs, *rhs).ok_or_else(error));
                            *lhs = out;
                            return VmResult::Ok(());
                        }
                        (Inline::Unsigned(lhs), Inline::Unsigned(rhs)) => {
                            let out = vm_try!(unsigned_op(*lhs, *rhs).ok_or_else(error));
                            *lhs = out;
                            return VmResult::Ok(());
                        }
                        (Inline::Float(lhs), Inline::Float(rhs)) => {
                            let out = float_op(*lhs, *rhs);
                            *lhs = out;
                            return VmResult::Ok(());
                        }
                        (lhs, rhs) => {
                            return err(VmErrorKind::UnsupportedBinaryOperation {
                                op: protocol.name,
                                lhs: lhs.type_info(),
                                rhs: rhs.type_info(),
                            });
                        }
                    },
                    (MutRepr::Inline(lhs), rhs) => {
                        return err(VmErrorKind::UnsupportedBinaryOperation {
                            op: protocol.name,
                            lhs: lhs.type_info(),
                            rhs: vm_try!(rhs.type_info()),
                        });
                    }
                    _ => {}
                }

                TargetFallback::Value(lhs.clone(), rhs.clone())
            }
            TargetValue::Fallback(fallback) => fallback,
        };

        self.target_fallback_assign(fallback, protocol)
    }

    /// Execute a fallback operation.
    #[cfg_attr(feature = "bench", inline(never))]
    fn target_fallback_assign(
        &mut self,
        fallback: TargetFallback<'_>,
        protocol: Protocol,
    ) -> VmResult<()> {
        match fallback {
            TargetFallback::Value(lhs, rhs) => {
                let mut args = DynGuardedArgs::new((&rhs,));

                if let CallResult::Unsupported(lhs) = vm_try!(self.call_instance_fn(
                    Isolated::None,
                    lhs,
                    protocol,
                    &mut args,
                    Output::discard()
                )) {
                    return err(VmErrorKind::UnsupportedBinaryOperation {
                        op: protocol.name,
                        lhs: vm_try!(lhs.type_info()),
                        rhs: vm_try!(rhs.type_info()),
                    });
                };

                VmResult::Ok(())
            }
            TargetFallback::Field(lhs, hash, rhs) => {
                let mut args = DynGuardedArgs::new((&rhs,));

                if let CallResult::Unsupported(lhs) = vm_try!(self.call_field_fn(
                    protocol,
                    lhs.clone(),
                    hash,
                    &mut args,
                    Output::discard()
                )) {
                    return err(VmErrorKind::UnsupportedObjectSlotIndexGet {
                        target: vm_try!(lhs.type_info()),
                    });
                }

                VmResult::Ok(())
            }
            TargetFallback::Index(lhs, index, rhs) => {
                let mut args = DynGuardedArgs::new((&rhs,));

                if let CallResult::Unsupported(lhs) = vm_try!(self.call_index_fn(
                    protocol,
                    lhs.clone(),
                    index,
                    &mut args,
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
    #[cfg_attr(feature = "bench", inline(never))]
    fn internal_num(
        &mut self,
        protocol: Protocol,
        error: fn() -> VmErrorKind,
        signed_op: fn(i64, i64) -> Option<i64>,
        unsigned_op: fn(u64, u64) -> Option<u64>,
        float_op: fn(f64, f64) -> f64,
        lhs: InstAddress,
        rhs: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        let rhs = vm_try!(self.stack.at(rhs));
        let lhs = vm_try!(self.stack.at(lhs));

        'fallback: {
            let inline = match (vm_try!(lhs.as_ref_repr()), vm_try!(rhs.as_ref_repr())) {
                (RefRepr::Inline(lhs), RefRepr::Inline(rhs)) => match (lhs, rhs) {
                    (Inline::Signed(lhs), Inline::Signed(rhs)) => {
                        Inline::Signed(vm_try!(signed_op(*lhs, *rhs).ok_or_else(error)))
                    }
                    (Inline::Unsigned(lhs), Inline::Unsigned(rhs)) => {
                        Inline::Unsigned(vm_try!(unsigned_op(*lhs, *rhs).ok_or_else(error)))
                    }
                    (Inline::Float(lhs), Inline::Float(rhs)) => Inline::Float(float_op(*lhs, *rhs)),
                    (lhs, rhs) => {
                        return err(VmErrorKind::UnsupportedBinaryOperation {
                            op: protocol.name,
                            lhs: lhs.type_info(),
                            rhs: rhs.type_info(),
                        });
                    }
                },
                (RefRepr::Inline(lhs), rhs) => {
                    return err(VmErrorKind::UnsupportedBinaryOperation {
                        op: protocol.name,
                        lhs: lhs.type_info(),
                        rhs: vm_try!(rhs.type_info()),
                    });
                }
                _ => {
                    break 'fallback;
                }
            };

            vm_try!(out.store(&mut self.stack, inline));
            return VmResult::Ok(());
        }

        let lhs = lhs.clone();
        let rhs = rhs.clone();

        let mut args = DynGuardedArgs::new((&rhs,));

        if let CallResult::Unsupported(lhs) =
            vm_try!(self.call_instance_fn(Isolated::None, lhs, protocol, &mut args, out))
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
    #[cfg_attr(feature = "bench", inline(never))]
    fn internal_infallible_bitwise_bool(
        &mut self,
        protocol: Protocol,
        signed_op: fn(i64, i64) -> i64,
        unsigned_op: fn(u64, u64) -> u64,
        byte_op: fn(u8, u8) -> u8,
        bool_op: fn(bool, bool) -> bool,
        lhs: InstAddress,
        rhs: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        let lhs = vm_try!(self.stack.at(lhs));
        let rhs = vm_try!(self.stack.at(rhs));

        'fallback: {
            let inline = match (vm_try!(lhs.as_ref_repr()), vm_try!(rhs.as_ref_repr())) {
                (RefRepr::Inline(lhs), RefRepr::Inline(rhs)) => match (lhs, rhs) {
                    (Inline::Signed(lhs), Inline::Signed(rhs)) => {
                        Inline::Signed(signed_op(*lhs, *rhs))
                    }
                    (Inline::Unsigned(lhs), Inline::Unsigned(rhs)) => {
                        Inline::Unsigned(unsigned_op(*lhs, *rhs))
                    }
                    (Inline::Byte(lhs), Inline::Byte(rhs)) => Inline::Byte(byte_op(*lhs, *rhs)),
                    (Inline::Bool(lhs), Inline::Bool(rhs)) => Inline::Bool(bool_op(*lhs, *rhs)),
                    (lhs, rhs) => {
                        return err(VmErrorKind::UnsupportedBinaryOperation {
                            op: protocol.name,
                            lhs: lhs.type_info(),
                            rhs: rhs.type_info(),
                        });
                    }
                },
                (RefRepr::Inline(lhs), rhs) => {
                    return err(VmErrorKind::UnsupportedBinaryOperation {
                        op: protocol.name,
                        lhs: lhs.type_info(),
                        rhs: vm_try!(rhs.type_info()),
                    });
                }
                _ => {
                    break 'fallback;
                }
            };

            vm_try!(out.store(&mut self.stack, inline));
            return VmResult::Ok(());
        };

        let lhs = lhs.clone();
        let rhs = rhs.clone();

        let mut args = DynGuardedArgs::new((&rhs,));

        if let CallResult::Unsupported(lhs) =
            vm_try!(self.call_instance_fn(Isolated::None, lhs, protocol, &mut args, out))
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
        signed_op: fn(&mut i64, i64),
        unsigned_op: fn(&mut u64, u64),
        byte_op: fn(&mut u8, u8),
        bool_op: fn(&mut bool, bool),
        rhs: InstAddress,
    ) -> VmResult<()> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs, rhs) {
            TargetValue::Same(value) => {
                match vm_try!(value.as_mut_repr()) {
                    MutRepr::Inline(Inline::Signed(value)) => {
                        let rhs = *value;
                        signed_op(value, rhs);
                        return VmResult::Ok(());
                    }
                    MutRepr::Inline(Inline::Unsigned(value)) => {
                        let rhs = *value;
                        unsigned_op(value, rhs);
                        return VmResult::Ok(());
                    }
                    MutRepr::Inline(Inline::Byte(value)) => {
                        let rhs = *value;
                        byte_op(value, rhs);
                        return VmResult::Ok(());
                    }
                    MutRepr::Inline(Inline::Bool(value)) => {
                        let rhs = *value;
                        bool_op(value, rhs);
                        return VmResult::Ok(());
                    }
                    MutRepr::Inline(value) => {
                        return err(VmErrorKind::UnsupportedBinaryOperation {
                            op: protocol.name,
                            lhs: value.type_info(),
                            rhs: value.type_info(),
                        });
                    }
                    _ => {}
                }

                TargetFallback::Value(value.clone(), value.clone())
            }
            TargetValue::Pair(lhs, rhs) => {
                match (vm_try!(lhs.as_mut_repr()), vm_try!(rhs.as_ref_repr())) {
                    (MutRepr::Inline(lhs), RefRepr::Inline(rhs)) => match (lhs, rhs) {
                        (Inline::Signed(lhs), Inline::Signed(rhs)) => {
                            signed_op(lhs, *rhs);
                            return VmResult::Ok(());
                        }
                        (Inline::Unsigned(lhs), Inline::Unsigned(rhs)) => {
                            unsigned_op(lhs, *rhs);
                            return VmResult::Ok(());
                        }
                        (Inline::Byte(lhs), Inline::Byte(rhs)) => {
                            byte_op(lhs, *rhs);
                            return VmResult::Ok(());
                        }
                        (Inline::Bool(lhs), Inline::Bool(rhs)) => {
                            bool_op(lhs, *rhs);
                            return VmResult::Ok(());
                        }
                        (lhs, rhs) => {
                            return err(VmErrorKind::UnsupportedBinaryOperation {
                                op: protocol.name,
                                lhs: lhs.type_info(),
                                rhs: rhs.type_info(),
                            });
                        }
                    },
                    (MutRepr::Inline(lhs), rhs) => {
                        return err(VmErrorKind::UnsupportedBinaryOperation {
                            op: protocol.name,
                            lhs: lhs.type_info(),
                            rhs: vm_try!(rhs.type_info()),
                        });
                    }
                    _ => {}
                }

                TargetFallback::Value(lhs.clone(), rhs.clone())
            }
            TargetValue::Fallback(fallback) => fallback,
        };

        self.target_fallback_assign(fallback, protocol)
    }

    fn internal_bitwise(
        &mut self,
        protocol: Protocol,
        error: fn() -> VmErrorKind,
        signed_op: fn(i64, u32) -> Option<i64>,
        unsigned_op: fn(u64, u32) -> Option<u64>,
        byte_op: fn(u8, u32) -> Option<u8>,
        lhs: InstAddress,
        rhs: InstAddress,
        out: Output,
    ) -> VmResult<()> {
        let (lhs, rhs) = 'fallback: {
            let inline = 'inline: {
                match vm_try!(self.stack.pair(lhs, rhs)) {
                    Pair::Same(value) => {
                        if let MutRepr::Inline(lhs) = vm_try!(value.as_mut_repr()) {
                            match lhs {
                                Inline::Signed(value) => {
                                    let shift =
                                        vm_try!(u32::try_from(*value).ok().ok_or_else(error));
                                    let value = vm_try!(signed_op(*value, shift).ok_or_else(error));
                                    break 'inline Inline::Signed(value);
                                }
                                Inline::Unsigned(value) => {
                                    let shift =
                                        vm_try!(u32::try_from(*value).ok().ok_or_else(error));
                                    let value =
                                        vm_try!(unsigned_op(*value, shift).ok_or_else(error));
                                    break 'inline Inline::Unsigned(value);
                                }
                                value => {
                                    return err(VmErrorKind::UnsupportedBinaryOperation {
                                        op: protocol.name,
                                        lhs: value.type_info(),
                                        rhs: value.type_info(),
                                    });
                                }
                            }
                        };

                        break 'fallback (value.clone(), value.clone());
                    }
                    Pair::Pair(lhs, rhs) => {
                        match (vm_try!(lhs.as_mut_repr()), vm_try!(rhs.as_ref_repr())) {
                            (MutRepr::Inline(lhs), RefRepr::Inline(rhs)) => match (lhs, rhs) {
                                (Inline::Signed(lhs), rhs) => {
                                    let rhs = vm_try!(rhs.try_as_integer());
                                    let value = vm_try!(signed_op(*lhs, rhs).ok_or_else(error));
                                    break 'inline Inline::Signed(value);
                                }
                                (Inline::Unsigned(lhs), rhs) => {
                                    let rhs = vm_try!(rhs.try_as_integer());
                                    let value = vm_try!(unsigned_op(*lhs, rhs).ok_or_else(error));
                                    break 'inline Inline::Unsigned(value);
                                }
                                (Inline::Byte(lhs), rhs) => {
                                    let rhs = vm_try!(rhs.try_as_integer());
                                    let value = vm_try!(byte_op(*lhs, rhs).ok_or_else(error));
                                    break 'inline Inline::Byte(value);
                                }
                                (lhs, rhs) => {
                                    return err(VmErrorKind::UnsupportedBinaryOperation {
                                        op: protocol.name,
                                        lhs: lhs.type_info(),
                                        rhs: rhs.type_info(),
                                    });
                                }
                            },
                            (MutRepr::Inline(lhs), rhs) => {
                                return err(VmErrorKind::UnsupportedBinaryOperation {
                                    op: protocol.name,
                                    lhs: lhs.type_info(),
                                    rhs: vm_try!(rhs.type_info()),
                                });
                            }
                            _ => {}
                        }

                        break 'fallback (lhs.clone(), rhs.clone());
                    }
                }
            };

            vm_try!(out.store(&mut self.stack, inline));
            return VmResult::Ok(());
        };

        let mut args = DynGuardedArgs::new((&rhs,));

        if let CallResult::Unsupported(lhs) =
            vm_try!(self.call_instance_fn(Isolated::None, lhs, protocol, &mut args, out))
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
        signed_op: fn(i64, u32) -> Option<i64>,
        unsigned_op: fn(u64, u32) -> Option<u64>,
        byte_op: fn(u8, u32) -> Option<u8>,
        rhs: InstAddress,
    ) -> VmResult<()> {
        let lhs;
        let mut guard;

        let fallback = match target_value!(self, target, guard, lhs, rhs) {
            TargetValue::Same(value) => {
                match vm_try!(value.as_mut_repr()) {
                    MutRepr::Inline(Inline::Signed(value)) => {
                        let shift = vm_try!(u32::try_from(*value).ok().ok_or_else(error));
                        let out = vm_try!(signed_op(*value, shift).ok_or_else(error));
                        *value = out;
                        return VmResult::Ok(());
                    }
                    MutRepr::Inline(Inline::Unsigned(value)) => {
                        let shift = vm_try!(u32::try_from(*value).ok().ok_or_else(error));
                        let out = vm_try!(unsigned_op(*value, shift).ok_or_else(error));
                        *value = out;
                        return VmResult::Ok(());
                    }
                    MutRepr::Inline(value) => {
                        return err(VmErrorKind::UnsupportedBinaryOperation {
                            op: protocol.name,
                            lhs: value.type_info(),
                            rhs: value.type_info(),
                        });
                    }
                    _ => {}
                }

                TargetFallback::Value(value.clone(), value.clone())
            }
            TargetValue::Pair(lhs, rhs) => {
                match (vm_try!(lhs.as_mut_repr()), vm_try!(rhs.as_ref_repr())) {
                    (MutRepr::Inline(lhs), RefRepr::Inline(rhs)) => match (lhs, rhs) {
                        (Inline::Signed(lhs), rhs) => {
                            let rhs = vm_try!(rhs.try_as_integer());
                            let out = vm_try!(signed_op(*lhs, rhs).ok_or_else(error));
                            *lhs = out;
                            return VmResult::Ok(());
                        }
                        (Inline::Unsigned(lhs), rhs) => {
                            let rhs = vm_try!(rhs.try_as_integer());
                            let out = vm_try!(unsigned_op(*lhs, rhs).ok_or_else(error));
                            *lhs = out;
                            return VmResult::Ok(());
                        }
                        (Inline::Byte(lhs), rhs) => {
                            let rhs = vm_try!(rhs.try_as_integer());
                            let out = vm_try!(byte_op(*lhs, rhs).ok_or_else(error));
                            *lhs = out;
                            return VmResult::Ok(());
                        }
                        (lhs, rhs) => {
                            return err(VmErrorKind::UnsupportedBinaryOperation {
                                op: protocol.name,
                                lhs: lhs.type_info(),
                                rhs: rhs.type_info(),
                            });
                        }
                    },
                    (MutRepr::Inline(lhs), rhs) => {
                        return err(VmErrorKind::UnsupportedBinaryOperation {
                            op: protocol.name,
                            lhs: lhs.type_info(),
                            rhs: vm_try!(rhs.type_info()),
                        });
                    }
                    _ => {}
                }

                TargetFallback::Value(lhs.clone(), rhs.clone())
            }
            TargetValue::Fallback(fallback) => fallback,
        };

        self.target_fallback_assign(fallback, protocol)
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_await(&mut self, addr: InstAddress) -> VmResult<Future> {
        VmResult::Ok(vm_try!(vm_try!(self.stack.at(addr)).clone().into_future()))
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_select(
        &mut self,
        addr: InstAddress,
        len: usize,
        value: Output,
    ) -> VmResult<Option<Select>> {
        let futures = futures_util::stream::FuturesUnordered::new();

        for (branch, value) in vm_try!(self.stack.slice_at(addr, len)).iter().enumerate() {
            let future = vm_try!(value.clone().into_future_mut());

            if !future.is_completed() {
                futures.push(SelectFuture::new(self.ip + branch, future));
            }
        }

        if futures.is_empty() {
            vm_try!(value.store(&mut self.stack, ()));
            self.ip = self.ip.wrapping_add(len);
            return VmResult::Ok(None);
        }

        VmResult::Ok(Some(Select::new(futures)))
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_store(&mut self, value: InstValue, out: Output) -> VmResult<()> {
        vm_try!(out.store(&mut self.stack, value.into_value()));
        VmResult::Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_copy(&mut self, addr: InstAddress, out: Output) -> VmResult<()> {
        vm_try!(self.stack.copy(addr, out));
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
        *vm_try!(self.stack.at_mut(addr)) = Value::empty();
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

    /// Construct a new vec.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_vec(&mut self, addr: InstAddress, count: usize, out: Output) -> VmResult<()> {
        let vec = vm_try!(self.stack.slice_at_mut(addr, count));
        let vec = vm_try!(vec.iter_mut().map(take).try_collect::<alloc::Vec<Value>>());
        vm_try!(out.store(&mut self.stack, Vec::from(vec)));
        VmResult::Ok(())
    }

    /// Construct a new tuple.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple(&mut self, addr: InstAddress, count: usize, out: Output) -> VmResult<()> {
        let tuple = vm_try!(self.stack.slice_at_mut(addr, count));

        let tuple = vm_try!(tuple
            .iter_mut()
            .map(take)
            .try_collect::<alloc::Vec<Value>>());

        vm_try!(
            out.store(&mut self.stack, || VmResult::Ok(Mutable::Tuple(vm_try!(
                OwnedTuple::try_from(tuple)
            ))))
        );

        VmResult::Ok(())
    }

    /// Construct a new tuple with a fixed number of arguments.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_tuple_n(&mut self, addr: &[InstAddress], out: Output) -> VmResult<()> {
        let mut tuple = vm_try!(alloc::Vec::<Value>::try_with_capacity(addr.len()));

        for &arg in addr {
            let value = vm_try!(self.stack.at(arg)).clone();
            vm_try!(tuple.try_push(value));
        }

        vm_try!(
            out.store(&mut self.stack, || VmResult::Ok(Mutable::Tuple(vm_try!(
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
                out.clone_from(value);
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

        let value = match vm_try!(value.borrow_ref_repr()) {
            BorrowRefRepr::Inline(value) => match value {
                Inline::Bool(value) => Value::from(!value),
                Inline::Signed(value) => Value::from(!value),
                Inline::Byte(value) => Value::from(!value),
                actual => {
                    let operand = actual.type_info();
                    return err(VmErrorKind::UnsupportedUnaryOperation { op: "!", operand });
                }
            },
            actual => {
                let operand = actual.type_info();
                return err(VmErrorKind::UnsupportedUnaryOperation { op: "!", operand });
            }
        };

        vm_try!(out.store(&mut self.stack, value));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_neg(&mut self, addr: InstAddress, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr));

        let value = match vm_try!(value.borrow_ref_repr()) {
            BorrowRefRepr::Inline(value) => match value {
                Inline::Float(value) => Value::from(-value),
                Inline::Signed(value) => Value::from(-value),
                actual => {
                    let operand = actual.type_info();
                    return err(VmErrorKind::UnsupportedUnaryOperation { op: "-", operand });
                }
            },
            actual => {
                let operand = actual.type_info();
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
                    u64::checked_add,
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
                    u64::checked_sub,
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
                    u64::checked_mul,
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
                    u64::checked_div,
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
                    u64::checked_rem,
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
                    u64::bitand,
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
                    u64::bitxor,
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
                    u64::bitor,
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
                    i64::checked_shl,
                    u64::checked_shl,
                    u8::checked_shl,
                    lhs,
                    rhs,
                    out,
                ));
            }
            InstOp::Shr => {
                vm_try!(self.internal_bitwise(
                    Protocol::SHR,
                    || VmErrorKind::Underflow,
                    i64::checked_shr,
                    u64::checked_shr,
                    u8::checked_shr,
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
                let rhs = vm_try!(self.stack.at(rhs));
                let lhs = vm_try!(self.stack.at(lhs));

                let test = if let (Some(lhs), Some(rhs)) =
                    (vm_try!(lhs.as_inline()), vm_try!(rhs.as_inline()))
                {
                    vm_try!(lhs.partial_eq(rhs))
                } else {
                    let lhs = lhs.clone();
                    let rhs = rhs.clone();
                    vm_try!(Value::partial_eq_with(&lhs, &rhs, self))
                };

                vm_try!(out.store(&mut self.stack, test));
            }
            InstOp::Neq => {
                let rhs = vm_try!(self.stack.at(rhs));
                let lhs = vm_try!(self.stack.at(lhs));

                let test = if let (Some(lhs), Some(rhs)) =
                    (vm_try!(lhs.as_inline()), vm_try!(rhs.as_inline()))
                {
                    vm_try!(lhs.partial_eq(rhs))
                } else {
                    let lhs = lhs.clone();
                    let rhs = rhs.clone();
                    vm_try!(Value::partial_eq_with(&lhs, &rhs, self))
                };

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
                    u64::checked_add,
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
                    u64::checked_sub,
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
                    u64::checked_mul,
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
                    u64::checked_div,
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
                    u64::checked_rem,
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
                    ops::BitOrAssign::bitor_assign,
                    value,
                ));
            }
            InstAssignOp::Shl => {
                vm_try!(self.internal_bitwise_assign(
                    target,
                    Protocol::SHL_ASSIGN,
                    || VmErrorKind::Overflow,
                    i64::checked_shl,
                    u64::checked_shl,
                    u8::checked_shl,
                    value,
                ));
            }
            InstAssignOp::Shr => {
                vm_try!(self.internal_bitwise_assign(
                    target,
                    Protocol::SHR_ASSIGN,
                    || VmErrorKind::Underflow,
                    i64::checked_shr,
                    u64::checked_shr,
                    u8::checked_shr,
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

        'fallback: {
            let Some(field) = vm_try!(index.try_borrow_ref::<String>()) else {
                break 'fallback;
            };

            if vm_try!(Self::try_object_slot_index_set(target, &field, value)) {
                return VmResult::Ok(());
            }
        };

        let target = target.clone();
        let index = index.clone();
        let value = value.clone();

        let mut args = DynGuardedArgs::new((&index, &value));

        if let CallResult::Unsupported(target) = vm_try!(self.call_instance_fn(
            Isolated::None,
            target,
            Protocol::INDEX_SET,
            &mut args,
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
    #[tracing::instrument(skip(self, return_value))]
    fn op_return_internal(&mut self, return_value: Value) -> VmResult<Option<Output>> {
        let (exit, out) = vm_try!(self.pop_call_frame());

        let out = if let Some(out) = out {
            vm_try!(out.store(&mut self.stack, return_value));
            out
        } else {
            let addr = self.stack.addr();
            vm_try!(self.stack.push(return_value));
            addr.output()
        };

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
    fn op_return_unit(&mut self) -> VmResult<Option<Output>> {
        let (exit, out) = vm_try!(self.pop_call_frame());

        let out = if let Some(out) = out {
            vm_try!(out.store(&mut self.stack, ()));
            out
        } else {
            let addr = self.stack.addr();
            vm_try!(self.stack.push(()));
            addr.output()
        };

        VmResult::Ok(exit.then_some(out))
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_load_instance_fn(&mut self, addr: InstAddress, hash: Hash, out: Output) -> VmResult<()> {
        let instance = vm_try!(self.stack.at(addr));
        let ty = vm_try!(instance.type_hash());
        let hash = Hash::associated_function(ty, hash);
        vm_try!(out.store(&mut self.stack, || Type::new(hash)));
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

            match vm_try!(index.as_ref_repr()) {
                RefRepr::Inline(inline) => {
                    let index = vm_try!(inline.try_as_integer::<usize>());

                    if let Some(value) = vm_try!(Self::try_tuple_like_index_get(target, index)) {
                        break 'store value;
                    }
                }
                RefRepr::Any(value) => {
                    if let Some(index) = vm_try!(value.try_borrow_ref::<String>()) {
                        if let Some(value) =
                            vm_try!(Self::try_object_like_index_get(target, index.as_str()))
                        {
                            break 'store value;
                        }
                    }
                }
                _ => {}
            }

            let target = target.clone();
            let index = index.clone();

            let mut args = DynGuardedArgs::new((&index,));

            if let CallResult::Unsupported(target) = vm_try!(self.call_instance_fn(
                Isolated::None,
                target,
                Protocol::INDEX_GET,
                &mut args,
                out
            )) {
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
        let value = vm_try!(self.stack.at(value));
        let target = vm_try!(self.stack.at(target));

        if vm_try!(Self::try_tuple_like_index_set(target, index, value)) {
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
            vm_try!(self.call_index_fn(Protocol::GET, value, index, &mut (), out))
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
        let target = vm_try!(self.stack.at(target));
        let value = vm_try!(self.stack.at(value));
        let field = vm_try!(self.unit.lookup_string(slot));

        if vm_try!(Self::try_object_slot_index_set(target, field, value)) {
            return VmResult::Ok(());
        }

        let target = target.clone();
        let value = value.clone();

        let hash = field.hash();

        let mut args = DynGuardedArgs::new((value,));

        let result =
            vm_try!(self.call_field_fn(Protocol::SET, target, hash, &mut args, Output::discard()));

        if let CallResult::Unsupported(target) = result {
            return err(VmErrorKind::UnsupportedObjectSlotIndexSet {
                target: vm_try!(target.type_info()),
            });
        };

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

        if let CallResult::Unsupported(target) =
            vm_try!(self.try_object_slot_index_get(target, slot, out))
        {
            return err(VmErrorKind::UnsupportedObjectSlotIndexGet {
                target: vm_try!(target.type_info()),
            });
        }

        VmResult::Ok(())
    }

    /// Operation to allocate an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_object(&mut self, addr: InstAddress, slot: usize, out: Output) -> VmResult<()> {
        let keys = vm_try!(self
            .unit
            .lookup_object_keys(slot)
            .ok_or(VmErrorKind::MissingStaticObjectKeys { slot }));

        let mut object = vm_try!(Object::with_capacity(keys.len()));
        let values = vm_try!(self.stack.slice_at_mut(addr, keys.len()));

        for (key, value) in keys.iter().zip(values) {
            let key = vm_try!(String::try_from(key.as_str()));
            vm_try!(object.insert(key, take(value)));
        }

        vm_try!(out.store(&mut self.stack, Mutable::Object(object)));
        VmResult::Ok(())
    }

    /// Operation to allocate an object.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_range(&mut self, range: InstRange, out: Output) -> VmResult<()> {
        let value = match range {
            InstRange::RangeFrom { start } => {
                let s = vm_try!(self.stack.at(start)).clone();
                vm_try!(Value::new(RangeFrom::new(s.clone())))
            }
            InstRange::RangeFull => {
                vm_try!(Value::new(RangeFull::new()))
            }
            InstRange::RangeInclusive { start, end } => {
                let s = vm_try!(self.stack.at(start)).clone();
                let e = vm_try!(self.stack.at(end)).clone();
                vm_try!(Value::new(RangeInclusive::new(s.clone(), e.clone())))
            }
            InstRange::RangeToInclusive { end } => {
                let e = vm_try!(self.stack.at(end)).clone();
                vm_try!(Value::new(RangeToInclusive::new(e.clone())))
            }
            InstRange::RangeTo { end } => {
                let e = vm_try!(self.stack.at(end)).clone();
                vm_try!(Value::new(RangeTo::new(e.clone())))
            }
            InstRange::Range { start, end } => {
                let s = vm_try!(self.stack.at(start)).clone();
                let e = vm_try!(self.stack.at(end)).clone();
                vm_try!(Value::new(Range::new(s.clone(), e.clone())))
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

        vm_try!(out.store(
            &mut self.stack,
            Mutable::EmptyStruct(EmptyStruct { rtti: rtti.clone() })
        ));

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

        let mut data = vm_try!(Object::with_capacity(keys.len()));
        let values = vm_try!(self.stack.slice_at_mut(addr, keys.len()));

        for (key, value) in keys.iter().zip(values) {
            let key = vm_try!(String::try_from(key.as_str()));
            vm_try!(data.insert(key, take(value)));
        }

        vm_try!(out.store(
            &mut self.stack,
            Mutable::Struct(Struct {
                rtti: rtti.clone(),
                data,
            })
        ));

        VmResult::Ok(())
    }

    /// Operation to allocate a constant value from an array of values.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_const_construct(
        &mut self,
        addr: InstAddress,
        hash: Hash,
        count: usize,
        out: Output,
    ) -> VmResult<()> {
        let values = vm_try!(self.stack.slice_at_mut(addr, count));

        let Some(construct) = self.context.construct(hash) else {
            return err(VmErrorKind::MissingConstantConstructor { hash });
        };

        let value = vm_try!(construct.runtime_construct(values));
        vm_try!(out.store(&mut self.stack, value));
        VmResult::Ok(())
    }

    /// Operation to allocate an object variant.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_struct_variant(
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
        let values = vm_try!(self.stack.slice_at_mut(addr, keys.len()));

        for (key, value) in keys.iter().zip(values) {
            let key = vm_try!(String::try_from(key.as_str()));
            vm_try!(data.insert(key, take(value)));
        }

        vm_try!(out.store(
            &mut self.stack,
            Mutable::Variant(Variant::struct_(rtti.clone(), data))
        ));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_string(&mut self, slot: usize, out: Output) -> VmResult<()> {
        let string = vm_try!(self.unit.lookup_string(slot));
        vm_try!(out.store(&mut self.stack, string.as_str()));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_bytes(&mut self, slot: usize, out: Output) -> VmResult<()> {
        let bytes = vm_try!(self.unit.lookup_bytes(slot));
        vm_try!(out.store(&mut self.stack, bytes));
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

        vm_try!(out.store(&mut self.stack, f.string));
        VmResult::Ok(())
    }

    /// Push a format specification onto the stack.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_format(&mut self, addr: InstAddress, spec: FormatSpec, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr)).clone();
        vm_try!(out.store(&mut self.stack, || Format { value, spec }));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_is_unit(&mut self, addr: InstAddress, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr));
        let is_unit = matches!(vm_try!(value.as_inline()), Some(Inline::Unit));
        vm_try!(out.store(&mut self.stack, is_unit));
        VmResult::Ok(())
    }

    /// Perform the try operation on the given stack location.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_try(&mut self, addr: InstAddress, out: Output) -> VmResult<Option<Output>> {
        let result = 'out: {
            let value = {
                let value = vm_try!(self.stack.at(addr));

                if let BorrowRefRepr::Mutable(value) = vm_try!(value.borrow_ref_repr()) {
                    match &*value {
                        Mutable::Result(result) => break 'out vm_try!(result::result_try(result)),
                        Mutable::Option(option) => break 'out vm_try!(option::option_try(option)),
                        _ => {}
                    }
                }

                value.clone()
            };

            match vm_try!(self.try_call_protocol_fn(Protocol::TRY, value, &mut ())) {
                CallResultOnly::Ok(value) => vm_try!(ControlFlow::from_value(value)),
                CallResultOnly::Unsupported(target) => {
                    return err(VmErrorKind::UnsupportedTryOperand {
                        actual: vm_try!(target.type_info()),
                    })
                }
            }
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

        let is_match = match vm_try!(v.as_inline()) {
            Some(Inline::Byte(actual)) => *actual == value,
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_character(&mut self, addr: InstAddress, value: char, out: Output) -> VmResult<()> {
        let v = vm_try!(self.stack.at(addr));

        let is_match = match vm_try!(v.as_inline()) {
            Some(Inline::Char(actual)) => *actual == value,
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_unsigned(&mut self, addr: InstAddress, value: u64, out: Output) -> VmResult<()> {
        let v = vm_try!(self.stack.at(addr));

        let is_match = match vm_try!(v.as_inline()) {
            Some(Inline::Unsigned(actual)) => *actual == value,
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_signed(&mut self, addr: InstAddress, value: i64, out: Output) -> VmResult<()> {
        let v = vm_try!(self.stack.at(addr));

        let is_match = match vm_try!(v.as_inline()) {
            Some(Inline::Signed(actual)) => *actual == value,
            _ => false,
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_bool(&mut self, addr: InstAddress, value: bool, out: Output) -> VmResult<()> {
        let v = vm_try!(self.stack.at(addr));

        let is_match = match vm_try!(v.as_inline()) {
            Some(Inline::Bool(actual)) => *actual == value,
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

        let is_match = 'out: {
            let Some(actual) = vm_try!(v.try_borrow_ref::<String>()) else {
                break 'out false;
            };

            let string = vm_try!(self.unit.lookup_string(slot));
            actual.as_str() == string.as_str()
        };

        vm_try!(out.store(&mut self.stack, is_match));
        VmResult::Ok(())
    }

    /// Test if the top of stack is equal to the string at the given static
    /// bytes slot.
    #[cfg_attr(feature = "bench", inline(never))]
    fn op_eq_bytes(&mut self, addr: InstAddress, slot: usize, out: Output) -> VmResult<()> {
        let v = vm_try!(self.stack.at(addr));

        let is_match = 'out: {
            let Some(value) = vm_try!(v.try_borrow_ref::<Bytes>()) else {
                break 'out false;
            };

            let bytes = vm_try!(self.unit.lookup_bytes(slot));
            value.as_slice() == bytes
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
            match vm_try!(value.borrow_ref_repr()) {
                BorrowRefRepr::Mutable(value) => match &*value {
                    Mutable::Variant(variant) => {
                        break 'out variant.rtti().hash == variant_hash;
                    }
                    _ => {
                        break 'out false;
                    }
                },
                BorrowRefRepr::Any(any) => {
                    if any.type_hash() != enum_hash {
                        break 'out false;
                    }
                }
                _ => {
                    break 'out false;
                }
            }

            let value = value.clone();

            match vm_try!(self.try_call_protocol_fn(
                Protocol::IS_VARIANT,
                value,
                &mut Some((index,))
            )) {
                CallResultOnly::Ok(value) => vm_try!(bool::from_value(value)),
                CallResultOnly::Unsupported(..) => false,
            }
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

        let is_match = match vm_try!(value.borrow_ref_repr()) {
            BorrowRefRepr::Inline(value) => match (type_check, value) {
                (TypeCheck::Unit, Inline::Unit) => true,
                _ => false,
            },
            BorrowRefRepr::Mutable(value) => match (type_check, &*value) {
                (TypeCheck::Tuple, Mutable::Tuple(..)) => true,
                (TypeCheck::Vec, Mutable::Vec(..)) => true,
                (TypeCheck::Result(v), Mutable::Result(result)) => match (v, result) {
                    (0, Ok(..)) => true,
                    (1, Err(..)) => true,
                    _ => false,
                },
                (TypeCheck::Option(v), Mutable::Option(option)) => match (v, option) {
                    (0, Some(..)) => true,
                    (1, None) => true,
                    _ => false,
                },
                (TypeCheck::GeneratorState(v), Mutable::GeneratorState(state)) => {
                    match (v, state) {
                        (0, Complete(..)) => true,
                        (1, Yielded(..)) => true,
                        _ => false,
                    }
                }
                _ => false,
            },
            BorrowRefRepr::Any(..) => false,
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

        let is_match = match vm_try!(value.borrow_ref_repr()) {
            BorrowRefRepr::Mutable(value) => match &*value {
                Mutable::Object(object) => {
                    let keys = vm_try!(self
                        .unit
                        .lookup_object_keys(slot)
                        .ok_or(VmErrorKind::MissingStaticObjectKeys { slot }));

                    test(object, keys, exact)
                }
                _ => false,
            },
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
        vm_try!(out.store(&mut self.stack, Mutable::Function(function)));
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

        vm_try!(out.store(&mut self.stack, Mutable::Function(function)));
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
                vm_try!(self.call_offset_fn(offset, call, addr, args, Isolated::None, out));
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

                let tuple = vm_try!(self.stack.slice_at_mut(addr, args));
                let tuple = vm_try!(tuple.iter_mut().map(take).try_collect());

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

                let tuple = vm_try!(self.stack.slice_at_mut(addr, args));
                let tuple = vm_try!(tuple.iter_mut().map(take).try_collect());

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
        vm_try!(self.call_offset_fn(offset, call, addr, args, Isolated::None, out));
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
        let instance = vm_try!(self.stack.at(addr));
        let type_hash = vm_try!(instance.type_hash());
        let hash = Hash::associated_function(type_hash, hash);

        if let Some(handler) = self.context.function(hash) {
            vm_try!(self.called_function_hook(hash));
            vm_try!(handler(&mut self.stack, addr, args, out));
            return VmResult::Ok(());
        }

        if let Some(UnitFn::Offset {
            offset,
            call,
            args: expected,
            ..
        }) = self.unit.function(hash)
        {
            vm_try!(self.called_function_hook(hash));
            vm_try!(check_args(args, expected));
            vm_try!(self.call_offset_fn(offset, call, addr, args, Isolated::None, out));
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
        let function = vm_try!(self.stack.at(function));

        if let Some(value) = vm_try!(function.as_inline()) {
            let ty = match value {
                Inline::Type(ty) => *ty,
                actual => {
                    return err(VmErrorKind::UnsupportedCallFn {
                        actual: actual.type_info(),
                    });
                }
            };

            vm_try!(self.op_call(ty.into_hash(), addr, args, out));
            return VmResult::Ok(None);
        }

        let function = function.clone();
        let function = vm_try!(function.borrow_ref_repr());

        match function {
            BorrowRefRepr::Mutable(value) => match &*value {
                Mutable::Function(function) => function.call_with_vm(self, addr, args, out),
                actual => err(VmErrorKind::UnsupportedCallFn {
                    actual: actual.type_info(),
                }),
            },
            actual => err(VmErrorKind::UnsupportedCallFn {
                actual: actual.type_info(),
            }),
        }
    }

    #[cfg_attr(feature = "bench", inline(never))]
    fn op_iter_next(&mut self, addr: InstAddress, jump: usize, out: Output) -> VmResult<()> {
        let value = vm_try!(self.stack.at(addr));

        let some = match vm_try!(value.borrow_ref_repr()) {
            BorrowRefRepr::Mutable(value) => match &*value {
                Mutable::Option(option) => match option {
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
        let _guard = crate::runtime::env::Guard::new(self.context.clone(), self.unit.clone(), None);
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
        let _guard =
            crate::runtime::env::Guard::new(self.context.clone(), self.unit.clone(), diagnostics);

        let mut budget = budget::acquire();

        loop {
            if !budget.take() {
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
                    if let Some(out) = vm_try!(self.op_return_unit()) {
                        return VmResult::Ok(VmHalt::Exited(out.as_addr()));
                    }
                }
                Inst::Await { addr, out } => {
                    let future = vm_try!(self.op_await(addr));
                    return VmResult::Ok(VmHalt::Awaited(Awaited::Future(future, out)));
                }
                Inst::Select { addr, len, value } => {
                    if let Some(select) = vm_try!(self.op_select(addr, len, value)) {
                        return VmResult::Ok(VmHalt::Awaited(Awaited::Select(select, value)));
                    }
                }
                Inst::LoadFn { hash, out } => {
                    vm_try!(self.op_load_fn(hash, out));
                }
                Inst::Store { value, out } => {
                    vm_try!(self.op_store(value, out));
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
                Inst::Vec { addr, count, out } => {
                    vm_try!(self.op_vec(addr, count, out));
                }
                Inst::Tuple { addr, count, out } => {
                    vm_try!(self.op_tuple(addr, count, out));
                }
                Inst::Tuple1 { addr, out } => {
                    vm_try!(self.op_tuple_n(&addr[..], out));
                }
                Inst::Tuple2 { addr, out } => {
                    vm_try!(self.op_tuple_n(&addr[..], out));
                }
                Inst::Tuple3 { addr, out } => {
                    vm_try!(self.op_tuple_n(&addr[..], out));
                }
                Inst::Tuple4 { addr, out } => {
                    vm_try!(self.op_tuple_n(&addr[..], out));
                }
                Inst::Environment { addr, count, out } => {
                    vm_try!(self.op_environment(addr, count, out));
                }
                Inst::Object { addr, slot, out } => {
                    vm_try!(self.op_object(addr, slot, out));
                }
                Inst::Range { range, out } => {
                    vm_try!(self.op_range(range, out));
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
                Inst::ConstConstruct {
                    addr,
                    hash,
                    count,
                    out,
                } => {
                    vm_try!(self.op_const_construct(addr, hash, count, out));
                }
                Inst::StructVariant {
                    addr,
                    hash,
                    slot,
                    out,
                } => {
                    vm_try!(self.op_struct_variant(addr, hash, slot, out));
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
                Inst::EqUnsigned { addr, value, out } => {
                    vm_try!(self.op_eq_unsigned(addr, value, out));
                }
                Inst::EqSigned { addr, value, out } => {
                    vm_try!(self.op_eq_signed(addr, value, out));
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
    pub top: usize,
    /// Indicates that the call frame is isolated and should force an exit into
    /// the vm execution context.
    pub isolated: Isolated,
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
