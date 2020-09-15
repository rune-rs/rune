use crate::future::SelectFuture;
use crate::unit::UnitFn;
use crate::{
    Args, Awaited, BorrowMut, Bytes, Call, Context, FromValue, Function, Future, Generator,
    GuardedArgs, Hash, Inst, InstOp, InstTarget, IntoHash, Object, Panic, Select, Shared, Stack,
    Stream, Tuple, TypeCheck, TypedObject, Unit, Value, VariantObject, VmError, VmErrorKind,
    VmExecution, VmHalt, VmIntegerRepr,
};
use std::fmt;
use std::mem;
use std::sync::Arc;

macro_rules! target_value {
    ($vm:ident, $target:expr, $guard:ident, $lhs:ident) => {{
        let rhs = $vm.stack.pop()?;

        let lhs = match $target {
            InstTarget::Offset(offset) => $vm.stack.at_offset_mut(offset)?,
            InstTarget::TupleField(index) => {
                $lhs = $vm.stack.pop()?;

                if let Some(value) = Vm::try_tuple_like_index_get_mut(&$lhs, index)? {
                    $guard = value;
                    &mut *$guard
                } else {
                    return Err(VmError::from(VmErrorKind::UnsupportedTupleIndexGet {
                        target: $lhs.type_info()?,
                    }));
                }
            }
            InstTarget::Field(field) => {
                let field = $vm.unit.lookup_string(field)?;
                $lhs = $vm.stack.pop()?;

                if let Some(value) = Vm::try_object_like_index_get_mut(&$lhs, field)? {
                    $guard = value;
                    &mut *$guard
                } else {
                    return Err(VmError::from(VmErrorKind::UnsupportedTupleIndexGet {
                        target: $lhs.type_info()?,
                    }));
                }
            }
        };

        (lhs, rhs)
    }};
}

/// A stack which references variables indirectly from a slab.
#[derive(Debug, Clone)]
pub struct Vm {
    /// Context associated with virtual machine.
    context: Arc<Context>,
    /// Unit associated with virtual machine.
    unit: Arc<Unit>,
    /// The current instruction pointer.
    ip: usize,
    /// The current stack.
    stack: Stack,
    /// Frames relative to the stack.
    call_frames: Vec<CallFrame>,
}

impl Vm {
    /// Construct a new runestick virtual machine.
    pub const fn new(context: Arc<Context>, unit: Arc<Unit>) -> Self {
        Self::new_with_stack(context, unit, Stack::new())
    }

    /// Construct a new runestick virtual machine.
    pub const fn new_with_stack(context: Arc<Context>, unit: Arc<Unit>, stack: Stack) -> Self {
        Self {
            context,
            unit,
            ip: 0,
            stack,
            call_frames: Vec::new(),
        }
    }

    /// Run the given vm to completion.
    ///
    /// If any async instructions are encountered, this will error.
    pub fn complete(self) -> Result<Value, VmError> {
        let mut execution = VmExecution::new(self);
        Ok(execution.complete()?)
    }

    /// Run the given vm to completion with support for async functions.
    pub async fn async_complete(self) -> Result<Value, VmError> {
        let mut execution = VmExecution::new(self);
        execution.async_complete().await
    }

    /// Test if the virtual machine is the same context and unit as specified.
    pub fn is_same(&self, context: &Arc<Context>, unit: &Arc<Unit>) -> bool {
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
    pub fn context(&self) -> &Arc<Context> {
        &self.context
    }

    /// Access the underlying unit of the virtual machine.
    pub fn unit(&self) -> &Arc<Unit> {
        &self.unit
    }

    /// Reset this virtual machine, freeing all memory used.
    pub fn clear(&mut self) {
        self.ip = 0;
        self.stack.clear();
        self.call_frames.clear();
    }

    /// Access the current instruction pointer.
    pub fn ip(&self) -> usize {
        self.ip
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
    ///     let context = Arc::new(Context::with_default_modules()?);
    ///     let unit = Arc::new(Unit::default());
    ///     // NB: normally the unit would be created by compiling some source,
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
    pub fn execute<A, N>(mut self, name: N, args: A) -> Result<VmExecution, VmError>
    where
        N: IntoHash,
        A: Args,
    {
        self.set_entrypoint(name, A::count())?;
        args.into_stack(&mut self.stack)?;
        Ok(VmExecution::new(self))
    }

    /// Call the given function immediately, returning the produced value.
    ///
    /// This function permits for using references since it doesn't defer its
    /// execution.
    ///
    /// # Panics
    ///
    /// If any of the arguments passed in are references, and that references is
    /// captured somewhere in the call as [`StrongMut<T>`] or [`StrongRef<T>`]
    /// this call will panic as we are trying to free the metadata relatedc to
    /// the reference.
    ///
    /// [`StrongMut<T>`]: crate::StrongMut
    /// [`StrongRef<T>`]: crate::StrongRef
    pub fn call<A, N>(mut self, name: N, args: A) -> Result<Value, VmError>
    where
        N: IntoHash,
        A: GuardedArgs,
    {
        self.set_entrypoint(name, A::count())?;

        // Safety: We hold onto the guard until the vm has completed.
        let guard = unsafe { args.unsafe_into_stack(&mut self.stack)? };

        let value = VmExecution::new(self).complete()?;

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
    /// captured somewhere in the call as [`StrongMut<T>`] or [`StrongRef<T>`]
    /// this call will panic as we are trying to free the metadata relatedc to
    /// the reference.
    ///
    /// [`StrongMut<T>`]: crate::StrongMut
    /// [`StrongRef<T>`]: crate::StrongRef
    pub async fn async_call<A, N>(mut self, name: N, args: A) -> Result<Value, VmError>
    where
        N: IntoHash,
        A: GuardedArgs,
    {
        self.set_entrypoint(name, A::count())?;

        // Safety: We hold onto the guard until the vm has completed.
        let guard = unsafe { args.unsafe_into_stack(&mut self.stack)? };

        let value = VmExecution::new(self).complete()?;

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
        N: IntoHash,
    {
        let hash = name.into_hash();

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

    fn op_await(&mut self) -> Result<Shared<Future>, VmError> {
        let value = self.stack.pop()?;

        match self.try_into_future(value)? {
            Ok(future) => Ok(future),
            Err(value) => Err(VmError::from(VmErrorKind::UnsupportedAwait {
                actual: value.type_info()?,
            })),
        }
    }

    fn op_select(&mut self, len: usize) -> Result<Option<Select>, VmError> {
        let futures = futures_util::stream::FuturesUnordered::new();

        let arguments = self.stack.drain_stack_top(len)?.collect::<Vec<_>>();

        for (branch, value) in arguments.into_iter().enumerate() {
            let future = match self.try_into_future(value)? {
                Ok(future) => future.into_mut()?,
                Err(value) => {
                    return Err(VmError::from(VmErrorKind::UnsupportedAwait {
                        actual: value.type_info()?,
                    }));
                }
            };

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

    /// Helper function to call an instance function.
    fn call_instance_fn<H, A>(&mut self, target: &Value, hash: H, args: A) -> Result<bool, VmError>
    where
        H: IntoHash,
        A: Args,
    {
        let count = A::count() + 1;
        let hash = Hash::instance_function(target.type_of()?, hash.into_hash());

        if let Some(UnitFn::Offset {
            offset,
            call,
            args: expected,
        }) = self.unit.lookup(hash)
        {
            Self::check_args(count, expected)?;
            self.stack.push(target.clone());
            args.into_stack(&mut self.stack)?;
            self.call_offset_fn(offset, call, count)?;
            return Ok(true);
        }

        let handler = match self.context.lookup(hash) {
            Some(handler) => handler,
            None => return Ok(false),
        };

        self.stack.push(target.clone());
        args.into_stack(&mut self.stack)?;

        handler(&mut self.stack, count)?;
        Ok(true)
    }

    /// Helper function to call an external getter.
    fn call_getter<H, A>(&mut self, target: &Value, hash: H, args: A) -> Result<bool, VmError>
    where
        H: IntoHash,
        A: Args,
    {
        let count = A::count() + 1;
        let hash = Hash::getter(target.type_of()?, hash.into_hash());

        let handler = match self.context.lookup(hash) {
            Some(handler) => handler,
            None => return Ok(false),
        };

        args.into_stack(&mut self.stack)?;

        self.stack.push(target.clone());
        handler(&mut self.stack, count)?;
        Ok(true)
    }

    /// Pop a number of values from the stack.
    fn op_popn(&mut self, n: usize) -> Result<(), VmError> {
        self.stack.popn(n)?;
        Ok(())
    }

    /// pop-and-jump-if-not instruction.
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
    fn op_clean(&mut self, n: usize) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        self.op_popn(n)?;
        self.stack.push(value);
        Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    fn op_copy(&mut self, offset: usize) -> Result<(), VmError> {
        let value = self.stack.at_offset(offset)?.clone();
        self.stack.push(value);
        Ok(())
    }

    #[inline]
    fn op_drop(&mut self, offset: usize) -> Result<(), VmError> {
        let _ = self.stack.at_offset(offset)?;
        Ok(())
    }

    /// Duplicate the value at the top of the stack.
    fn op_dup(&mut self) -> Result<(), VmError> {
        let value = self.stack.last()?.clone();
        self.stack.push(value);
        Ok(())
    }

    /// Copy a value from a position relative to the top of the stack, to the
    /// top of the stack.
    fn op_replace(&mut self, offset: usize) -> Result<(), VmError> {
        let mut value = self.stack.pop()?;
        let stack_value = self.stack.at_offset_mut(offset)?;
        mem::swap(stack_value, &mut value);
        Ok(())
    }

    fn internal_boolean_ops(
        &mut self,
        int_op: impl FnOnce(i64, i64) -> bool,
        float_op: impl FnOnce(f64, f64) -> bool,
        op: &'static str,
    ) -> Result<(), VmError> {
        let rhs = self.stack.pop()?;
        let lhs = self.stack.pop()?;

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

    fn op_gt(&mut self) -> Result<(), VmError> {
        self.internal_boolean_ops(|a, b| a > b, |a, b| a > b, ">")?;
        Ok(())
    }

    fn op_gte(&mut self) -> Result<(), VmError> {
        self.internal_boolean_ops(|a, b| a >= b, |a, b| a >= b, ">=")?;
        Ok(())
    }

    fn op_lt(&mut self) -> Result<(), VmError> {
        self.internal_boolean_ops(|a, b| a < b, |a, b| a < b, "<")?;
        Ok(())
    }

    fn op_lte(&mut self) -> Result<(), VmError> {
        self.internal_boolean_ops(|a, b| a <= b, |a, b| a <= b, "<=")?;
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

    /// Optimized equality implementation.
    #[inline]
    fn op_eq(&mut self) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        self.stack.push(Value::value_ptr_eq(&a, &b)?);
        Ok(())
    }

    /// Optimized inequality implementation.
    #[inline]
    fn op_neq(&mut self) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        self.stack.push(!Value::value_ptr_eq(&a, &b)?);
        Ok(())
    }

    /// Perform a jump operation.
    #[inline]
    fn op_jump(&mut self, offset: isize) -> Result<(), VmError> {
        self.modify_ip(offset)?;
        Ok(())
    }

    /// Perform a conditional jump operation.
    #[inline]
    fn op_jump_if(&mut self, offset: isize) -> Result<(), VmError> {
        if self.stack.pop()?.into_bool()? {
            self.modify_ip(offset)?;
        }

        Ok(())
    }

    /// Perform a conditional jump operation.
    #[inline]
    fn op_jump_if_not(&mut self, offset: isize) -> Result<(), VmError> {
        if !self.stack.pop()?.into_bool()? {
            self.modify_ip(offset)?;
        }

        Ok(())
    }

    /// Perform a branch-conditional jump operation.
    #[inline]
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
    #[inline]
    fn op_vec(&mut self, count: usize) -> Result<(), VmError> {
        let vec = self.stack.pop_sequence(count)?;
        self.stack.push(Shared::new(vec));
        Ok(())
    }

    /// Construct a new tuple.
    #[inline]
    fn op_tuple(&mut self, count: usize) -> Result<(), VmError> {
        let tuple = self.stack.pop_sequence(count)?;
        self.stack.push(Tuple::from(tuple));
        Ok(())
    }

    /// Push the tuple that is on top of the stack.
    #[inline]
    fn op_push_tuple(&mut self) -> Result<(), VmError> {
        let tuple = self.stack.pop()?.into_tuple()?;
        self.stack.extend(tuple.borrow_ref()?.iter().cloned());
        Ok(())
    }

    #[inline]
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

    #[inline]
    fn op_op(&mut self, op: InstOp) -> Result<(), VmError> {
        use std::convert::TryFrom as _;

        match op {
            InstOp::Add => {
                self.internal_num(
                    crate::ADD,
                    || VmError::from(VmErrorKind::Overflow),
                    i64::checked_add,
                    std::ops::Add::add,
                    "+",
                )?;
            }
            InstOp::Sub => {
                self.internal_num(
                    crate::SUB,
                    || VmError::from(VmErrorKind::Underflow),
                    i64::checked_sub,
                    std::ops::Sub::sub,
                    "-",
                )?;
            }
            InstOp::Mul => {
                self.internal_num(
                    crate::ADD,
                    || VmError::from(VmErrorKind::Overflow),
                    i64::checked_mul,
                    std::ops::Mul::mul,
                    "*",
                )?;
            }
            InstOp::Div => {
                self.internal_num(
                    crate::ADD,
                    || VmError::from(VmErrorKind::DivideByZero),
                    i64::checked_div,
                    std::ops::Div::div,
                    "+",
                )?;
            }
            InstOp::Rem => {
                self.internal_num(
                    crate::REM,
                    || VmError::from(VmErrorKind::DivideByZero),
                    i64::checked_rem,
                    std::ops::Rem::rem,
                    "%",
                )?;
            }
            InstOp::BitAnd => {
                self.internal_infallible_bitwise(crate::BIT_AND, std::ops::BitAnd::bitand, "&")?;
            }
            InstOp::BitXor => {
                self.internal_infallible_bitwise(crate::BIT_XOR, std::ops::BitXor::bitxor, "^")?;
            }
            InstOp::BitOr => {
                self.internal_infallible_bitwise(crate::BIT_OR, std::ops::BitOr::bitor, "|")?;
            }
            InstOp::Shl => {
                self.internal_bitwise(
                    crate::SHL,
                    || VmError::from(VmErrorKind::Overflow),
                    |a, b| a.checked_shl(u32::try_from(b).ok()?),
                    "<<",
                )?;
            }
            InstOp::Shr => {
                self.internal_infallible_bitwise(crate::SHR, std::ops::Shr::shr, ">>")?;
            }
        }

        Ok(())
    }

    #[inline]
    fn op_assign(&mut self, target: InstTarget, op: InstOp) -> Result<(), VmError> {
        use std::convert::TryFrom as _;

        match op {
            InstOp::Add => {
                self.internal_num_assign(
                    target,
                    crate::ADD_ASSIGN,
                    || VmError::from(VmErrorKind::Overflow),
                    i64::checked_add,
                    std::ops::Add::add,
                    "+=",
                )?;
            }
            InstOp::Sub => {
                self.internal_num_assign(
                    target,
                    crate::SUB_ASSIGN,
                    || VmError::from(VmErrorKind::Underflow),
                    i64::checked_sub,
                    std::ops::Sub::sub,
                    "-=",
                )?;
            }
            InstOp::Mul => {
                self.internal_num_assign(
                    target,
                    crate::MUL_ASSIGN,
                    || VmError::from(VmErrorKind::Overflow),
                    i64::checked_mul,
                    std::ops::Mul::mul,
                    "*=",
                )?;
            }
            InstOp::Div => {
                self.internal_num_assign(
                    target,
                    crate::DIV_ASSIGN,
                    || VmError::from(VmErrorKind::DivideByZero),
                    i64::checked_div,
                    std::ops::Div::div,
                    "/=",
                )?;
            }
            InstOp::Rem => {
                self.internal_num_assign(
                    target,
                    crate::REM_ASSIGN,
                    || VmError::from(VmErrorKind::DivideByZero),
                    i64::checked_rem,
                    std::ops::Rem::rem,
                    "%=",
                )?;
            }
            InstOp::BitAnd => {
                self.internal_infallible_bitwise_assign(
                    target,
                    crate::BIT_AND_ASSIGN,
                    std::ops::BitAndAssign::bitand_assign,
                    "&=",
                )?;
            }
            InstOp::BitXor => {
                self.internal_infallible_bitwise_assign(
                    target,
                    crate::BIT_XOR_ASSIGN,
                    std::ops::BitXorAssign::bitxor_assign,
                    "^=",
                )?;
            }
            InstOp::BitOr => {
                self.internal_infallible_bitwise_assign(
                    target,
                    crate::BIT_OR_ASSIGN,
                    std::ops::BitOrAssign::bitor_assign,
                    "|=",
                )?;
            }
            InstOp::Shl => {
                self.internal_bitwise_assign(
                    target,
                    crate::SHL_ASSIGN,
                    || VmError::from(VmErrorKind::Overflow),
                    |a, b| a.checked_shl(u32::try_from(b).ok()?),
                    "<<=",
                )?;
            }
            InstOp::Shr => {
                self.internal_infallible_bitwise_assign(
                    target,
                    crate::SHR_ASSIGN,
                    std::ops::ShrAssign::shr_assign,
                    ">>=",
                )?;
            }
        }

        Ok(())
    }

    /// Perform an index set operation.
    #[inline]
    fn op_index_set(&mut self) -> Result<(), VmError> {
        let target = self.stack.pop()?;
        let index = self.stack.pop()?;
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
                Value::TypedObject(typed_object) => {
                    let mut typed_object = typed_object.borrow_mut()?;

                    if let Some(v) = typed_object.object.get_mut(field) {
                        *v = value;
                        return Ok(());
                    }

                    return Err(VmError::from(VmErrorKind::MissingField {
                        field: field.to_owned(),
                        target: typed_object.type_info(),
                    }));
                }
                Value::VariantObject(variant_object) => {
                    let mut variant_object = variant_object.borrow_mut()?;

                    if let Some(v) = variant_object.object.get_mut(field) {
                        *v = value;
                        return Ok(());
                    }

                    return Err(VmError::from(VmErrorKind::MissingField {
                        field: field.to_owned(),
                        target: variant_object.type_info(),
                    }));
                }
                _ => break,
            }
        }

        if !self.call_instance_fn(&target, crate::INDEX_SET, (&index, &value))? {
            return Err(VmError::from(VmErrorKind::UnsupportedIndexSet {
                target: target.type_info()?,
                index: index.type_info()?,
                value: value.type_info()?,
            }));
        }

        Ok(())
    }

    #[inline]
    fn op_return(&mut self) -> Result<bool, VmError> {
        let return_value = self.stack.pop()?;
        let exit = self.pop_call_frame()?;
        self.stack.push(return_value);
        Ok(exit)
    }

    #[inline]
    fn op_return_unit(&mut self) -> Result<bool, VmError> {
        let exit = self.pop_call_frame()?;
        self.stack.push(());
        Ok(exit)
    }

    #[inline]
    fn op_load_instance_fn(&mut self, hash: Hash) -> Result<(), VmError> {
        let instance = self.stack.pop()?;
        let ty = instance.type_of()?;
        let hash = Hash::instance_function(ty, hash);
        self.stack.push(Value::Type(hash));
        Ok(())
    }

    /// Try to convert the given value into a future.
    ///
    /// Returns the value we failed to convert as an `Err` variant if we are
    /// unsuccessful.
    fn try_into_future(&mut self, value: Value) -> Result<Result<Shared<Future>, Value>, VmError> {
        match value {
            Value::Future(future) => Ok(Ok(future)),
            value => {
                if !self.call_instance_fn(&value, crate::INTO_FUTURE, ())? {
                    return Ok(Err(value));
                }

                if let Value::Future(future) = self.stack.pop()? {
                    return Ok(Ok(future));
                }

                Ok(Err(value))
            }
        }
    }

    /// Implementation of getting a string index on an object-like type.
    fn try_object_like_index_get(&mut self, target: &Value, field: &str) -> Result<bool, VmError> {
        let value = match &target {
            Value::Object(target) => target.borrow_ref()?.get(field).cloned(),
            Value::TypedObject(target) => target.borrow_ref()?.object.get(field).cloned(),
            Value::VariantObject(target) => target.borrow_ref()?.object.get(field).cloned(),
            _ => return Ok(false),
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

        self.stack.push(value);
        Ok(true)
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
            Value::TypedTuple(typed_tuple) => {
                let typed_tuple = typed_tuple.borrow_ref()?;
                typed_tuple.tuple.get(index).cloned()
            }
            Value::TupleVariant(variant_tuple) => {
                let variant_tuple = variant_tuple.borrow_ref()?;
                variant_tuple.tuple.get(index).cloned()
            }
            _ => return Ok(None),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return Err(VmError::from(VmErrorKind::MissingIndex {
                    target: target.type_info()?,
                    index: VmIntegerRepr::Usize(index),
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
            Value::TypedTuple(typed_tuple) => {
                let typed_tuple = typed_tuple.borrow_mut()?;

                BorrowMut::try_map(typed_tuple, |typed_tuple| typed_tuple.get_mut(index))
            }
            Value::TupleVariant(variant_tuple) => {
                let variant_tuple = variant_tuple.borrow_mut()?;

                BorrowMut::try_map(variant_tuple, |variant_tuple| {
                    variant_tuple.tuple.get_mut(index)
                })
            }
            _ => return Ok(None),
        };

        let value = match value {
            Some(value) => value,
            None => {
                return Err(VmError::from(VmErrorKind::MissingIndex {
                    target: target.type_info()?,
                    index: VmIntegerRepr::Usize(index),
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
            Value::TypedObject(target) => {
                let target = target.borrow_mut()?;
                BorrowMut::try_map(target, |target| target.get_mut(field))
            }
            Value::VariantObject(target) => {
                let target = target.borrow_mut()?;
                BorrowMut::try_map(target, |target| target.get_mut(field))
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
            Value::TypedTuple(typed_tuple) => {
                let mut typed_tuple = typed_tuple.borrow_mut()?;

                if let Some(target) = typed_tuple.tuple.get_mut(index) {
                    *target = value;
                    return Ok(true);
                }

                Ok(false)
            }
            Value::TupleVariant(variant_tuple) => {
                let mut variant_tuple = variant_tuple.borrow_mut()?;

                if let Some(target) = variant_tuple.tuple.get_mut(index) {
                    *target = value;
                    return Ok(true);
                }

                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Perform an index get operation.
    #[inline]
    fn op_index_get(&mut self) -> Result<(), VmError> {
        let target = self.stack.pop()?;
        let index = self.stack.pop()?;

        // This is a useful pattern.
        #[allow(clippy::never_loop)]
        loop {
            match &index {
                Value::String(string) => {
                    let string_ref = string.borrow_ref()?;

                    if self.try_object_like_index_get(&target, string_ref.as_str())? {
                        return Ok(());
                    }
                }
                Value::StaticString(string) => {
                    if self.try_object_like_index_get(&target, string.as_ref())? {
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
                                index: VmIntegerRepr::I64(*index),
                            }));
                        }
                    };

                    if let Some(value) = Self::try_tuple_like_index_get(&target, index)? {
                        self.stack.push(value);
                        return Ok(());
                    }
                }
                _ => break,
            };
        }

        if !self.call_instance_fn(&target, crate::INDEX_GET, (&index,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedIndexGet {
                target: target.type_info()?,
                index: index.type_info()?,
            }));
        }

        Ok(())
    }

    /// Perform an index get operation specialized for tuples.
    #[inline]
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
    #[inline]
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
    #[inline]
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
            Value::TypedObject(typed_object) => {
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
            Value::VariantObject(variant_object) => {
                let variant_object = variant_object.borrow_ref()?;

                match variant_object.object.get(&***index).cloned() {
                    Some(value) => Some(value),
                    None => {
                        return Err(VmError::from(VmErrorKind::ObjectIndexMissing {
                            slot: string_slot,
                        }));
                    }
                }
            }
            target => {
                let hash = index.hash();

                if self.call_getter(target, hash, ())? {
                    Some(self.stack.pop()?)
                } else {
                    None
                }
            }
        })
    }

    /// Perform a specialized index get operation on an object.
    #[inline]
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

    /// Perform a specialized index get operation on an object.
    #[inline]
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
    #[inline]
    fn op_object(&mut self, slot: usize) -> Result<(), VmError> {
        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or_else(|| VmError::from(VmErrorKind::MissingStaticObjectKeys { slot }))?;

        let mut object = Object::with_capacity(keys.len());
        let values = self.stack.drain_stack_top(keys.len())?;

        for (key, value) in keys.iter().zip(values) {
            object.insert(key.clone(), value);
        }

        self.stack.push(Shared::new(object));
        Ok(())
    }

    /// Operation to allocate an object.
    #[inline]
    fn op_typed_object(&mut self, hash: Hash, slot: usize) -> Result<(), VmError> {
        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or_else(|| VmError::from(VmErrorKind::MissingStaticObjectKeys { slot }))?;

        let mut object = Object::with_capacity(keys.len());

        let values = self.stack.drain_stack_top(keys.len())?;

        for (key, value) in keys.iter().zip(values) {
            object.insert(key.clone(), value);
        }

        self.stack.push(TypedObject::new(hash, object));
        Ok(())
    }

    /// Operation to allocate an object.
    #[inline]
    fn op_variant_object(
        &mut self,
        enum_hash: Hash,
        hash: Hash,
        slot: usize,
    ) -> Result<(), VmError> {
        let keys = self
            .unit
            .lookup_object_keys(slot)
            .ok_or_else(|| VmError::from(VmErrorKind::MissingStaticObjectKeys { slot }))?;

        let mut object = Object::with_capacity(keys.len());
        let values = self.stack.drain_stack_top(keys.len())?;

        for (key, value) in keys.iter().zip(values) {
            object.insert(key.clone(), value);
        }

        self.stack.push(VariantObject {
            enum_hash,
            hash,
            object,
        });

        Ok(())
    }

    #[inline]
    fn op_string(&mut self, slot: usize) -> Result<(), VmError> {
        let string = self.unit.lookup_string(slot)?;
        self.stack.push(string.clone());
        Ok(())
    }

    #[inline]
    fn op_bytes(&mut self, slot: usize) -> Result<(), VmError> {
        let bytes = self.unit.lookup_bytes(slot)?.to_owned();
        self.stack.push(Bytes::from_vec(bytes));
        Ok(())
    }

    /// Optimize operation to perform string concatenation.
    #[inline]
    fn op_string_concat(&mut self, len: usize, size_hint: usize) -> Result<(), VmError> {
        let mut buf = String::with_capacity(size_hint);
        let values = self.stack.drain_stack_top(len)?.collect::<Vec<_>>();

        for value in values {
            match value {
                Value::String(string) => {
                    buf.push_str(&*string.borrow_ref()?);
                }
                Value::StaticString(string) => {
                    buf.push_str(string.as_ref());
                }
                Value::Integer(integer) => {
                    let mut buffer = itoa::Buffer::new();
                    buf.push_str(buffer.format(integer));
                }
                Value::Float(float) => {
                    let mut buffer = ryu::Buffer::new();
                    buf.push_str(buffer.format(float));
                }
                actual => {
                    let b = Shared::new(std::mem::take(&mut buf));

                    if !self.call_instance_fn(
                        &actual,
                        crate::STRING_DISPLAY,
                        (Value::String(b.clone()),),
                    )? {
                        return Err(VmError::from(VmErrorKind::MissingProtocol {
                            protocol: crate::STRING_DISPLAY,
                            actual: actual.type_info()?,
                        }));
                    }

                    let value = fmt::Result::from_value(self.stack.pop()?)?;

                    if let Err(fmt::Error) = value {
                        return Err(VmError::from(VmErrorKind::FormatError));
                    }

                    buf = b.take()?;
                }
            }
        }

        self.stack.push(buf);
        Ok(())
    }

    #[inline]
    fn op_unwrap(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let value = match value {
            Value::Option(option) => match option.take()? {
                Some(value) => value,
                None => {
                    return Err(VmError::from(VmErrorKind::UnsupportedUnwrapNone));
                }
            },
            Value::Result(result) => match result.take()? {
                Ok(value) => value,
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

    /// Internal implementation of the instance check.
    fn is_instance(&mut self) -> Result<bool, VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;

        let hash = match b {
            Value::Type(hash) => hash,
            _ => {
                return Err(VmError::from(VmErrorKind::UnsupportedIs {
                    value: a.type_info()?,
                    test_type: b.type_info()?,
                }));
            }
        };

        Ok(*a.type_of()? == hash)
    }

    #[inline]
    fn op_is(&mut self) -> Result<(), VmError> {
        let is_instance = self.is_instance()?;
        self.stack.push(is_instance);
        Ok(())
    }

    #[inline]
    fn op_is_not(&mut self) -> Result<(), VmError> {
        let is_instance = self.is_instance()?;
        self.stack.push(!is_instance);
        Ok(())
    }

    #[inline]
    fn op_is_unit(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        self.stack.push(matches!(value, Value::Unit));
        Ok(())
    }

    /// Test if the top of the stack is an error.
    #[inline]
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

    fn internal_boolean_op(
        &mut self,
        bool_op: impl FnOnce(bool, bool) -> bool,
        op: &'static str,
    ) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;

        let out = match (a, b) {
            (Value::Bool(a), Value::Bool(b)) => bool_op(a, b),
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

    /// Operation associated with `and` instruction.
    #[inline]
    fn op_and(&mut self) -> Result<(), VmError> {
        self.internal_boolean_op(|a, b| a && b, "&&")?;
        Ok(())
    }

    /// Operation associated with `or` instruction.
    #[inline]
    fn op_or(&mut self) -> Result<(), VmError> {
        self.internal_boolean_op(|a, b| a || b, "||")?;
        Ok(())
    }

    #[inline]
    fn op_eq_byte(&mut self, byte: u8) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.stack.push(match value {
            Value::Byte(actual) => actual == byte,
            _ => false,
        });

        Ok(())
    }

    #[inline]
    fn op_eq_character(&mut self, character: char) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.stack.push(match value {
            Value::Char(actual) => actual == character,
            _ => false,
        });

        Ok(())
    }

    #[inline]
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
    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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
            (TypeCheck::Type(hash), Value::TypedTuple(typed_tuple)) => {
                let typed_tuple = typed_tuple.borrow_ref()?;

                if typed_tuple.hash != hash {
                    return Ok(None);
                }

                Some(f(&*typed_tuple.tuple))
            }
            (TypeCheck::Variant(hash), Value::TupleVariant(variant_tuple)) => {
                let variant_tuple = variant_tuple.borrow_ref()?;

                if variant_tuple.hash != hash {
                    return Ok(None);
                }

                Some(f(&*variant_tuple.tuple))
            }
            (TypeCheck::Unit, Value::Unit) => Some(f(&[])),
            _ => None,
        })
    }

    #[inline]
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
            .ok_or_else(|| VmError::from(VmErrorKind::MissingStaticObjectKeys { slot }))?;

        match (type_check, value) {
            (TypeCheck::Object, Value::Object(object)) => {
                let object = object.borrow_ref()?;
                return Ok(Some(f(&*object, keys)));
            }
            (TypeCheck::Type(hash), Value::TypedObject(typed_object)) => {
                let typed_object = typed_object.borrow_ref()?;

                if typed_object.type_hash() == hash {
                    return Ok(Some(f(&typed_object.object, keys)));
                }
            }
            (TypeCheck::Variant(hash), Value::VariantObject(variant_object)) => {
                let variant_object = variant_object.borrow_ref()?;

                if variant_object.hash == hash {
                    return Ok(Some(f(&variant_object.object, keys)));
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

    fn call_offset_fn(&mut self, offset: usize, call: Call, args: usize) -> Result<(), VmError> {
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

    fn op_load_fn(&mut self, hash: Hash) -> Result<(), VmError> {
        let function = match self.unit.lookup(hash) {
            Some(info) => match info {
                UnitFn::Offset { offset, call, args } => Function::from_offset(
                    self.context.clone(),
                    self.unit.clone(),
                    offset,
                    call,
                    args,
                ),
                UnitFn::Tuple { hash, args } => Function::from_tuple(hash, args),
                UnitFn::TupleVariant {
                    enum_hash,
                    hash,
                    args,
                } => Function::from_variant_tuple(enum_hash, hash, args),
            },
            None => {
                let handler = self
                    .context
                    .lookup(hash)
                    .ok_or_else(|| VmError::from(VmErrorKind::MissingFunction { hash }))?;

                Function::from_handler(handler.clone())
            }
        };

        self.stack.push(Value::Function(Shared::new(function)));
        Ok(())
    }

    /// Construct a closure on the top of the stack.
    fn op_closure(&mut self, hash: Hash, count: usize) -> Result<(), VmError> {
        let info = self
            .unit
            .lookup(hash)
            .ok_or_else(|| VmError::from(VmErrorKind::MissingFunction { hash }))?;

        let (offset, call, args) = match info {
            UnitFn::Offset { offset, call, args } => (offset, call, args),
            _ => return Err(VmError::from(VmErrorKind::MissingFunction { hash })),
        };

        let environment = self.stack.pop_sequence(count)?;
        let environment = Shared::new(Tuple::from(environment));

        let function = Function::from_closure(
            self.context.clone(),
            self.unit.clone(),
            offset,
            call,
            args,
            environment,
        );

        self.stack.push(Value::Function(Shared::new(function)));
        Ok(())
    }

    /// Implementation of a function call.
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
                UnitFn::Tuple {
                    hash,
                    args: expected,
                } => {
                    Self::check_args(args, expected)?;
                    let tuple = self.stack.pop_sequence(args)?;
                    let value = Value::typed_tuple(hash, tuple);
                    self.stack.push(value);
                }
                UnitFn::TupleVariant {
                    enum_hash,
                    hash,
                    args: expected,
                } => {
                    Self::check_args(args, expected)?;
                    let tuple = self.stack.pop_sequence(args)?;
                    let value = Value::variant_tuple(enum_hash, hash, tuple);
                    self.stack.push(value);
                }
            },
            None => {
                let handler = self
                    .context
                    .lookup(hash)
                    .ok_or_else(|| VmError::from(VmErrorKind::MissingFunction { hash }))?;

                handler(&mut self.stack, args)?;
            }
        }

        Ok(())
    }

    #[inline]
    fn op_call_instance<H>(&mut self, hash: H, args: usize) -> Result<(), VmError>
    where
        H: IntoHash,
    {
        // NB: +1 to include the instance itself.
        let args = args + 1;
        let instance = self.stack.at_offset_from_top(args)?;
        let type_of = instance.type_of()?;
        let hash = Hash::instance_function(type_of, hash);

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

    /// Advance the instruction pointer.
    pub(crate) fn advance(&mut self) {
        self.ip = self.ip.overflowing_add(1).0;
    }

    /// Evaluate a single instruction.
    pub(crate) fn run_for(&mut self, mut limit: Option<usize>) -> Result<VmHalt, VmError> {
        loop {
            let inst = *self
                .unit
                .instruction_at(self.ip)
                .ok_or_else(|| VmError::from(VmErrorKind::IpOutOfBounds))?;

            log::trace!("{}: {}", self.ip, inst);

            match inst {
                Inst::Not => {
                    self.op_not()?;
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
                Inst::IndexGet => {
                    self.op_index_get()?;
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
                    self.stack.push(value.into_value());
                }
                Inst::Pop => {
                    self.stack.pop()?;
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
                Inst::Drop { offset } => {
                    self.op_drop(offset)?;
                }
                Inst::Dup => {
                    self.op_dup()?;
                }
                Inst::Replace { offset } => {
                    self.op_replace(offset)?;
                }
                Inst::Gt => {
                    self.op_gt()?;
                }
                Inst::Gte => {
                    self.op_gte()?;
                }
                Inst::Lt => {
                    self.op_lt()?;
                }
                Inst::Lte => {
                    self.op_lte()?;
                }
                Inst::Eq => {
                    self.op_eq()?;
                }
                Inst::Neq => {
                    self.op_neq()?;
                }
                Inst::Jump { offset } => {
                    self.op_jump(offset)?;
                }
                Inst::JumpIf { offset } => {
                    self.op_jump_if(offset)?;
                }
                Inst::JumpIfNot { offset } => {
                    self.op_jump_if_not(offset)?;
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
                Inst::PushTuple => {
                    self.op_push_tuple()?;
                }
                Inst::Object { slot } => {
                    self.op_object(slot)?;
                }
                Inst::TypedObject { hash, slot } => {
                    self.op_typed_object(hash, slot)?;
                }
                Inst::VariantObject {
                    enum_hash,
                    hash,
                    slot,
                } => {
                    self.op_variant_object(enum_hash, hash, slot)?;
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
                Inst::Is => {
                    self.op_is()?;
                }
                Inst::IsNot => {
                    self.op_is_not()?;
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
                Inst::And => {
                    self.op_and()?;
                }
                Inst::Or => {
                    self.op_or()?;
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
                Inst::Op { op } => {
                    self.op_op(op)?;
                }
                Inst::Assign { target, op } => {
                    self.op_assign(target, op)?;
                }
                Inst::Panic { reason } => {
                    return Err(VmError::from(VmErrorKind::Panic {
                        reason: Panic::from(reason),
                    }));
                }
            }

            self.advance();

            if let Some(limit) = &mut limit {
                if *limit <= 1 {
                    return Ok(VmHalt::Limited);
                }

                *limit -= 1;
            }
        }
    }

    fn internal_num_assign<H, E, I, F>(
        &mut self,
        target: InstTarget,
        hash: H,
        error: E,
        integer_op: I,
        float_op: F,
        op: &'static str,
    ) -> Result<(), VmError>
    where
        H: IntoHash,
        E: FnOnce() -> VmError,
        I: FnOnce(i64, i64) -> Option<i64>,
        F: FnOnce(f64, f64) -> f64,
    {
        let mut guard;
        let lhs;

        let (lhs, rhs) = target_value!(self, target, guard, lhs);

        let (lhs, rhs) = match (lhs, rhs) {
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
            (lhs, rhs) => (lhs.clone(), rhs),
        };

        if !self.call_instance_fn(&lhs, hash, (&rhs,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                op,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }));
        }

        self.stack.pop()?;
        Ok(())
    }

    /// Internal impl of a numeric operation.
    fn internal_num<H, E, I, F>(
        &mut self,
        hash: H,
        error: E,
        integer_op: I,
        float_op: F,
        op: &'static str,
    ) -> Result<(), VmError>
    where
        H: IntoHash,
        E: FnOnce() -> VmError,
        I: FnOnce(i64, i64) -> Option<i64>,
        F: FnOnce(f64, f64) -> f64,
    {
        let rhs = self.stack.pop()?;
        let lhs = self.stack.pop()?;

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

        if !self.call_instance_fn(&lhs, hash, (&rhs,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                op,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }));
        }

        Ok(())
    }

    /// Internal impl of a numeric operation.
    fn internal_infallible_bitwise<H, I>(
        &mut self,
        hash: H,
        integer_op: I,
        op: &'static str,
    ) -> Result<(), VmError>
    where
        H: IntoHash,
        I: FnOnce(i64, i64) -> i64,
    {
        let rhs = self.stack.pop()?;
        let lhs = self.stack.pop()?;

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                self.stack.push(integer_op(lhs, rhs));
                return Ok(());
            }
            (lhs, rhs) => (lhs, rhs),
        };

        if !self.call_instance_fn(&lhs, hash, (&rhs,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                op,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }));
        }

        Ok(())
    }

    fn internal_infallible_bitwise_assign<H, I>(
        &mut self,
        target: InstTarget,
        hash: H,
        integer_op: I,
        op: &'static str,
    ) -> Result<(), VmError>
    where
        H: IntoHash,
        I: FnOnce(&mut i64, i64),
    {
        let mut guard;
        let lhs;

        let (lhs, rhs) = target_value!(self, target, guard, lhs);

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                integer_op(lhs, rhs);
                return Ok(());
            }
            (lhs, rhs) => (lhs.clone(), rhs),
        };

        if !self.call_instance_fn(&lhs, hash, (&rhs,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                op,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }));
        }

        self.stack.pop()?;
        Ok(())
    }

    fn internal_bitwise<H, E, I>(
        &mut self,
        hash: H,
        error: E,
        integer_op: I,
        op: &'static str,
    ) -> Result<(), VmError>
    where
        H: IntoHash,
        E: FnOnce() -> VmError,
        I: FnOnce(i64, i64) -> Option<i64>,
    {
        let rhs = self.stack.pop()?;
        let lhs = self.stack.pop()?;

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                self.stack.push(integer_op(lhs, rhs).ok_or_else(error)?);
                return Ok(());
            }
            (lhs, rhs) => (lhs, rhs),
        };

        if !self.call_instance_fn(&lhs, hash, (&rhs,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                op,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }));
        }

        Ok(())
    }

    fn internal_bitwise_assign<H, E, I>(
        &mut self,
        target: InstTarget,
        hash: H,
        error: E,
        integer_op: I,
        op: &'static str,
    ) -> Result<(), VmError>
    where
        H: IntoHash,
        E: FnOnce() -> VmError,
        I: FnOnce(i64, i64) -> Option<i64>,
    {
        let mut guard;
        let lhs;

        let (lhs, rhs) = target_value!(self, target, guard, lhs);

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                let out = integer_op(*lhs, rhs).ok_or_else(error)?;
                *lhs = out;
                return Ok(());
            }
            (lhs, rhs) => (lhs.clone(), rhs),
        };

        if !self.call_instance_fn(&lhs, hash, (&rhs,))? {
            return Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                op,
                lhs: lhs.type_info()?,
                rhs: rhs.type_info()?,
            }));
        }

        Ok(())
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
