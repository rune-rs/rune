use crate::future::SelectFuture;
use crate::unit::{UnitFnCall, UnitFnKind};
use crate::{
    Bytes, Context, FnPtr, FromValue, Future, Generator, Hash, Inst, Integer, IntoArgs,
    IntoTypeHash, Object, Panic, Select, Shared, Stack, ToValue, Tuple, TypeCheck, TypedObject,
    Unit, Value, VariantObject, VmError, VmErrorKind, VmExecution,
};
use std::any;
use std::fmt;
use std::marker;
use std::mem;
use std::rc::Rc;

/// A stack which references variables indirectly from a slab.
#[derive(Debug, Clone)]
pub struct Vm {
    /// Context associated with virtual machine.
    context: Rc<Context>,
    /// Unit associated with virtual machine.
    unit: Rc<Unit>,
    /// The current instruction pointer.
    ip: usize,
    /// The current stack.
    stack: Stack,
    /// Frames relative to the stack.
    call_frames: Vec<CallFrame>,
}

impl Vm {
    /// Construct a new runestick virtual machine.
    pub const fn new(context: Rc<Context>, unit: Rc<Unit>) -> Self {
        Self::new_with_stack(context, unit, Stack::new())
    }

    /// Construct a new runestick virtual machine.
    pub const fn new_with_stack(context: Rc<Context>, unit: Rc<Unit>, stack: Stack) -> Self {
        Self {
            context,
            unit,
            ip: 0,
            stack,
            call_frames: Vec::new(),
        }
    }

    /// Run the given vm to completion.
    pub async fn run_to_completion(self) -> Result<Value, VmError> {
        let mut execution = VmExecution::of(self);
        execution.run_to_completion().await
    }

    /// Test if the virtual machine is the same context and unit as specified.
    pub fn is_same(&self, context: &Rc<Context>, unit: &Rc<Unit>) -> bool {
        Rc::ptr_eq(&self.context, context) && Rc::ptr_eq(&self.unit, unit)
    }

    /// Set  the current instruction pointer.
    #[inline]
    pub fn set_ip(&mut self, ip: usize) {
        self.ip = ip;
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
    pub fn context(&self) -> &Rc<Context> {
        &self.context
    }

    /// Access the underlying unit of the virtual machine.
    pub fn unit(&self) -> &Rc<Unit> {
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

    /// Iterate over the stack, producing the value associated with each stack
    /// item.
    pub fn iter_stack_debug(&self) -> impl Iterator<Item = &Value> + '_ {
        self.stack.iter()
    }

    /// Call the given function in the given compilation unit.
    pub fn call_function<A, N>(mut self, hash: N, args: A) -> Result<VmExecution, VmError>
    where
        N: IntoTypeHash,
        A: IntoArgs,
    {
        let hash = hash.into_type_hash();

        let function = self
            .unit
            .lookup(hash)
            .ok_or_else(|| VmError::from(VmErrorKind::MissingFunction { hash }))?;

        if function.signature.args != A::count() {
            return Err(VmError::from(VmErrorKind::ArgumentCountMismatch {
                actual: A::count(),
                expected: function.signature.args,
            }));
        }

        let offset = match function.kind {
            // NB: we ignore the calling convention.
            // everything is just async when called externally.
            UnitFnKind::Offset { offset, .. } => offset,
            _ => {
                return Err(VmError::from(VmErrorKind::MissingFunction { hash }));
            }
        };

        self.ip = offset;
        self.stack.clear();

        // Safety: we bind the lifetime of the arguments to the outgoing task,
        // ensuring that the task won't outlive any references passed in.
        args.into_args(&mut self.stack)?;
        Ok(VmExecution::of(self))
    }

    /// Run the given program on the virtual machine.
    pub fn run<T>(&mut self) -> Task<'_, T>
    where
        T: FromValue,
    {
        Task {
            vm: self,
            _marker: marker::PhantomData,
        }
    }

    fn op_await(&mut self) -> Result<Shared<Future>, VmError> {
        loop {
            let value = self.stack.pop()?;

            match value {
                Value::Future(future) => return Ok(future),
                value => {
                    if !self.call_instance_fn(&value, crate::INTO_FUTURE, ())? {
                        return Err(VmError::from(VmErrorKind::UnsupportedAwait {
                            actual: value.type_info()?,
                        }));
                    }
                }
            }
        }
    }

    fn op_select(&mut self, len: usize) -> Result<Option<Select>, VmError> {
        let futures = futures::stream::FuturesUnordered::new();

        for branch in 0..len {
            let future = self.stack.pop()?.into_future()?.owned_mut()?;

            if !future.is_completed() {
                futures.push(SelectFuture::new(branch, future));
            }
        }

        // NB: nothing to poll.
        if futures.is_empty() {
            self.stack.push(Value::Unit);
            return Ok(None);
        }

        Ok(Some(Select::new(futures)))
    }

    /// Helper function to call an instance function.
    fn call_instance_fn<H, A>(&mut self, target: &Value, hash: H, args: A) -> Result<bool, VmError>
    where
        H: IntoTypeHash,
        A: IntoArgs,
    {
        let count = A::count() + 1;
        let hash = Hash::instance_function(target.value_type()?, hash.into_type_hash());

        if let Some(info) = self.unit.lookup(hash) {
            if info.signature.args != count {
                return Err(VmError::from(VmErrorKind::ArgumentCountMismatch {
                    actual: count,
                    expected: info.signature.args,
                }));
            }

            if let UnitFnKind::Offset { offset, call } = &info.kind {
                let offset = *offset;
                let call = *call;

                args.into_args(&mut self.stack)?;

                self.stack.push(target.clone());
                self.call_offset_fn(offset, call, count)?;
                return Ok(true);
            }
        }

        let handler = match self.context.lookup(hash) {
            Some(handler) => handler,
            None => return Ok(false),
        };

        args.into_args(&mut self.stack)?;

        self.stack.push(target.clone());
        handler(&mut self.stack, count)?;
        Ok(true)
    }

    /// Helper function to call an external getter.
    fn call_getter<H, A>(&mut self, target: &Value, hash: H, args: A) -> Result<bool, VmError>
    where
        H: IntoTypeHash,
        A: IntoArgs,
    {
        let count = A::count() + 1;
        let hash = Hash::getter(target.value_type()?, hash.into_type_hash());

        let handler = match self.context.lookup(hash) {
            Some(handler) => handler,
            None => return Ok(false),
        };

        args.into_args(&mut self.stack)?;

        self.stack.push(target.clone());
        handler(&mut self.stack, count)?;
        Ok(true)
    }

    /// Pop a number of values from the stack.
    fn op_popn(&mut self, n: usize) -> Result<(), VmError> {
        self.stack.popn(n)?;
        Ok(())
    }

    /// pop-and-jump-if instruction.
    fn op_pop_and_jump_if(&mut self, count: usize, offset: isize) -> Result<(), VmError> {
        if !self.stack.pop()?.into_bool()? {
            return Ok(());
        }

        self.stack.popn(count)?;
        self.modify_ip(offset)?;
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

        self.stack.push(Value::Bool(out));
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
        let stack_top = self.stack.swap_stack_top(args)?;

        self.call_frames.push(CallFrame {
            ip: self.ip,
            stack_top,
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

        self.stack.pop_stack_top(frame.stack_top)?;
        self.ip = frame.ip;
        Ok(false)
    }

    /// Pop the last value on the stack and evaluate it as `T`.
    fn pop_decode<T>(&mut self) -> Result<T, VmError>
    where
        T: FromValue,
    {
        let value = self.stack.pop()?;

        let value = match T::from_value(value) {
            Ok(value) => value,
            Err(error) => {
                return Err(VmError::from(VmErrorKind::StackConversionError {
                    error,
                    to: any::type_name::<T>(),
                }));
            }
        };

        Ok(value)
    }

    /// Optimized function to test if two value pointers are deeply equal to
    /// each other.
    ///
    /// This is the basis for the eq operation (`==`).
    fn value_ptr_eq(&self, a: &Value, b: &Value) -> Result<bool, VmError> {
        Ok(match (a, b) {
            (Value::Unit, Value::Unit) => true,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Vec(a), Value::Vec(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                if a.len() != b.len() {
                    return Ok(false);
                }

                for (a, b) in a.iter().zip(b.iter()) {
                    if !self.value_ptr_eq(a, b)? {
                        return Ok(false);
                    }
                }

                true
            }
            (Value::Object(a), Value::Object(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                if a.len() != b.len() {
                    return Ok(false);
                }

                for (key, a) in a.iter() {
                    let b = match b.get(key) {
                        Some(b) => b,
                        None => return Ok(false),
                    };

                    if !self.value_ptr_eq(a, b)? {
                        return Ok(false);
                    }
                }

                true
            }
            (Value::String(a), Value::String(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;
                *a == *b
            }
            (Value::StaticString(a), Value::String(b)) => {
                let b = b.borrow_ref()?;
                ***a == *b
            }
            (Value::String(a), Value::StaticString(b)) => {
                let a = a.borrow_ref()?;
                *a == ***b
            }
            // fast string comparison: exact string slot.
            (Value::StaticString(a), Value::StaticString(b)) => ***a == ***b,
            // fast external comparison by slot.
            // TODO: implement ptr equals.
            // (Value::Any(a), Value::Any(b)) => a == b,
            _ => false,
        })
    }

    /// Optimized equality implementation.
    #[inline]
    fn op_eq(&mut self) -> Result<(), VmError> {
        let a = self.stack.pop()?;
        let b = self.stack.pop()?;
        self.stack.push(Value::Bool(self.value_ptr_eq(&a, &b)?));
        Ok(())
    }

    /// Optimized inequality implementation.
    #[inline]
    fn op_neq(&mut self) -> Result<(), VmError> {
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        self.stack.push(Value::Bool(!self.value_ptr_eq(&a, &b)?));
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
        let mut vec = Vec::with_capacity(count);

        for _ in 0..count {
            vec.push(self.stack.pop()?);
        }

        let value = Value::Vec(Shared::new(vec));
        self.stack.push(value);
        Ok(())
    }

    /// Construct a new tuple.
    #[inline]
    fn op_tuple(&mut self, count: usize) -> Result<(), VmError> {
        let mut tuple = Vec::with_capacity(count);

        for _ in 0..count {
            tuple.push(self.stack.pop()?);
        }

        let value = Value::Tuple(Shared::new(Tuple::from(tuple)));
        self.stack.push(value);
        Ok(())
    }

    #[inline]
    fn op_not(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        let value = match value {
            Value::Bool(value) => Value::Bool(!value),
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

    /// Internal impl of a numeric operation.
    fn internal_numeric_op<H, E, I, F>(
        &mut self,
        hash: H,
        error: E,
        integer_op: I,
        float_op: F,
        op: &'static str,
    ) -> Result<(), VmError>
    where
        H: IntoTypeHash,
        E: Copy + FnOnce() -> VmError,
        I: FnOnce(i64, i64) -> Option<i64>,
        F: FnOnce(f64, f64) -> f64,
    {
        let rhs = self.stack.pop()?;
        let lhs = self.stack.pop()?;

        let (lhs, rhs) = match (lhs, rhs) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                let out = integer_op(lhs, rhs).ok_or_else(error)?;
                self.stack.push(Value::Integer(out));
                return Ok(());
            }
            (Value::Float(lhs), Value::Float(rhs)) => {
                let out = float_op(lhs, rhs);
                self.stack.push(Value::Float(out));
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

    #[inline]
    fn op_add(&mut self) -> Result<(), VmError> {
        self.internal_numeric_op(
            crate::ADD,
            || VmError::from(VmErrorKind::Overflow),
            i64::checked_add,
            std::ops::Add::add,
            "+",
        )?;
        Ok(())
    }

    #[inline]
    fn op_sub(&mut self) -> Result<(), VmError> {
        self.internal_numeric_op(
            crate::SUB,
            || VmError::from(VmErrorKind::Underflow),
            i64::checked_sub,
            std::ops::Sub::sub,
            "-",
        )?;
        Ok(())
    }

    #[inline]
    fn op_mul(&mut self) -> Result<(), VmError> {
        self.internal_numeric_op(
            crate::ADD,
            || VmError::from(VmErrorKind::Overflow),
            i64::checked_mul,
            std::ops::Mul::mul,
            "*",
        )?;
        Ok(())
    }

    #[inline]
    fn op_div(&mut self) -> Result<(), VmError> {
        self.internal_numeric_op(
            crate::ADD,
            || VmError::from(VmErrorKind::DivideByZero),
            i64::checked_div,
            std::ops::Div::div,
            "+",
        )?;
        Ok(())
    }

    #[inline]
    fn op_rem(&mut self) -> Result<(), VmError> {
        self.internal_numeric_op(
            crate::REM,
            || VmError::from(VmErrorKind::DivideByZero),
            i64::checked_rem,
            std::ops::Rem::rem,
            "%",
        )?;
        Ok(())
    }

    fn internal_op_assign<H, E, I, F>(
        &mut self,
        offset: usize,
        hash: H,
        error: E,
        integer_op: I,
        float_op: F,
        op: &'static str,
    ) -> Result<(), VmError>
    where
        H: IntoTypeHash,
        E: Copy + FnOnce() -> VmError,
        I: FnOnce(i64, i64) -> Option<i64>,
        F: FnOnce(f64, f64) -> f64,
    {
        let rhs = self.stack.pop()?;
        let lhs = self.stack.at_offset_mut(offset)?;

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

    #[inline]
    fn op_add_assign(&mut self, offset: usize) -> Result<(), VmError> {
        self.internal_op_assign(
            offset,
            crate::ADD_ASSIGN,
            || VmError::from(VmErrorKind::Overflow),
            i64::checked_add,
            std::ops::Add::add,
            "+=",
        )?;
        Ok(())
    }

    #[inline]
    fn op_sub_assign(&mut self, offset: usize) -> Result<(), VmError> {
        self.internal_op_assign(
            offset,
            crate::SUB_ASSIGN,
            || VmError::from(VmErrorKind::Underflow),
            i64::checked_sub,
            std::ops::Sub::sub,
            "-=",
        )?;
        Ok(())
    }

    #[inline]
    fn op_mul_assign(&mut self, offset: usize) -> Result<(), VmError> {
        self.internal_op_assign(
            offset,
            crate::MUL_ASSIGN,
            || VmError::from(VmErrorKind::Overflow),
            i64::checked_mul,
            std::ops::Mul::mul,
            "*=",
        )?;
        Ok(())
    }

    #[inline]
    fn op_div_assign(&mut self, offset: usize) -> Result<(), VmError> {
        self.internal_op_assign(
            offset,
            crate::DIV_ASSIGN,
            || VmError::from(VmErrorKind::DivideByZero),
            i64::checked_div,
            std::ops::Div::div,
            "/=",
        )?;
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
        self.stack.push(Value::Unit);
        Ok(exit)
    }

    #[inline]
    fn op_load_instance_fn(&mut self, hash: Hash) -> Result<(), VmError> {
        let instance = self.stack.pop()?;
        let ty = instance.value_type()?;
        let hash = Hash::instance_function(ty, hash);
        self.stack.push(Value::Type(hash));
        Ok(())
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
            Value::VariantTuple(variant_tuple) => {
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
                    index: Integer::Usize(index),
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
            Value::VariantTuple(variant_tuple) => {
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
                                index: Integer::I64(*index),
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

                match typed_object.object.get(&***index).cloned() {
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
    fn op_object_slot_index_get(&mut self, string_slot: usize) -> Result<(), VmError> {
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
    fn op_object_slot_index_get_at(
        &mut self,
        offset: usize,
        string_slot: usize,
    ) -> Result<(), VmError> {
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

        for key in keys {
            let value = self.stack.pop()?;
            object.insert(key.clone(), value);
        }

        let value = Value::Object(Shared::new(object));
        self.stack.push(value);
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

        for key in keys {
            let value = self.stack.pop()?;
            object.insert(key.clone(), value);
        }

        let object = TypedObject { hash, object };
        let value = Value::TypedObject(Shared::new(object));
        self.stack.push(value);
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

        for key in keys {
            let value = self.stack.pop()?;
            object.insert(key.clone(), value);
        }

        let object = VariantObject {
            enum_hash,
            hash,
            object,
        };
        let value = Value::VariantObject(Shared::new(object));
        self.stack.push(value);
        Ok(())
    }

    #[inline]
    fn op_string(&mut self, slot: usize) -> Result<(), VmError> {
        let string = self.unit.lookup_string(slot)?;
        let value = Value::StaticString(string.clone());
        self.stack.push(value);
        Ok(())
    }

    #[inline]
    fn op_bytes(&mut self, slot: usize) -> Result<(), VmError> {
        let bytes = self.unit.lookup_bytes(slot)?.to_owned();
        let value = Value::Bytes(Shared::new(Bytes::from_vec(bytes)));
        self.stack.push(value);
        Ok(())
    }

    /// Optimize operation to perform string concatenation.
    #[inline]
    fn op_string_concat(&mut self, len: usize, size_hint: usize) -> Result<(), VmError> {
        let mut buf = String::with_capacity(size_hint);

        for _ in 0..len {
            let value = self.stack.pop()?;

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

                    let value = self.pop_decode::<fmt::Result>()?;

                    if let Err(fmt::Error) = value {
                        return Err(VmError::from(VmErrorKind::FormatError));
                    }

                    buf = b.take()?;
                }
            }
        }

        self.stack.push(Value::String(Shared::new(buf)));
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

        Ok(a.value_type()? == hash)
    }

    #[inline]
    fn op_is(&mut self) -> Result<(), VmError> {
        let is_instance = self.is_instance()?;
        self.stack.push(Value::Bool(is_instance));
        Ok(())
    }

    #[inline]
    fn op_is_not(&mut self) -> Result<(), VmError> {
        let is_instance = self.is_instance()?;
        self.stack.push(Value::Bool(!is_instance));
        Ok(())
    }

    #[inline]
    fn op_is_unit(&mut self) -> Result<(), VmError> {
        let value = self.stack.pop()?;
        self.stack.push(Value::Bool(matches!(value, Value::Unit)));
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

        self.stack.push(Value::Bool(is_value));
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

        self.stack.push(Value::Bool(out));
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

        self.stack.push(Value::Bool(match value {
            Value::Byte(actual) => actual == byte,
            _ => false,
        }));

        Ok(())
    }

    #[inline]
    fn op_eq_character(&mut self, character: char) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.stack.push(Value::Bool(match value {
            Value::Char(actual) => actual == character,
            _ => false,
        }));

        Ok(())
    }

    #[inline]
    fn op_eq_integer(&mut self, integer: i64) -> Result<(), VmError> {
        let value = self.stack.pop()?;

        self.stack.push(Value::Bool(match value {
            Value::Integer(actual) => actual == integer,
            _ => false,
        }));

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
            (TypeCheck::Variant(hash), Value::VariantTuple(variant_tuple)) => {
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
        F: FnOnce(&Object<Value>, &[String]) -> O,
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

                if typed_object.hash == hash {
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
        let generator = Generator::new(vm);
        self.stack.push(Value::Generator(Shared::new(generator)));
        Ok(())
    }

    /// Construct a future from calling an async function.
    fn call_async_fn(&mut self, offset: usize, args: usize) -> Result<(), VmError> {
        let stack = self.stack.drain_stack_top(args)?.collect::<Stack>();
        let mut vm = Self::new_with_stack(self.context.clone(), self.unit.clone(), stack);
        vm.ip = offset;
        let future = Future::new(async move { vm.run_to_completion().await });
        self.stack.push(Value::Future(Shared::new(future)));
        Ok(())
    }

    fn call_offset_fn(
        &mut self,
        offset: usize,
        call: UnitFnCall,
        args: usize,
    ) -> Result<(), VmError> {
        match call {
            UnitFnCall::Immediate => {
                self.push_call_frame(offset, args)?;
            }
            UnitFnCall::Generator => {
                self.call_generator_fn(offset, args)?;
            }
            UnitFnCall::Async => {
                self.call_async_fn(offset, args)?;
            }
        }

        Ok(())
    }

    fn op_fn(&mut self, hash: Hash) -> Result<(), VmError> {
        let fn_ptr = match self.unit.lookup(hash) {
            Some(info) => {
                let args = info.signature.args;

                match &info.kind {
                    UnitFnKind::Offset { offset, call } => FnPtr::from_offset(
                        self.context.clone(),
                        self.unit.clone(),
                        *offset,
                        *call,
                        args,
                    ),
                    UnitFnKind::Tuple { hash } => FnPtr::from_tuple(*hash, args),
                    UnitFnKind::TupleVariant { enum_hash, hash } => {
                        FnPtr::from_variant_tuple(*enum_hash, *hash, args)
                    }
                }
            }
            None => {
                let handler = self
                    .context
                    .lookup(hash)
                    .ok_or_else(|| VmError::from(VmErrorKind::MissingFunction { hash }))?;

                FnPtr::from_handler(handler.clone())
            }
        };

        self.stack.push(Value::FnPtr(Shared::new(fn_ptr)));
        Ok(())
    }

    /// Construct a closure on the top of the stack.
    fn op_closure(&mut self, hash: Hash, count: usize) -> Result<(), VmError> {
        let info = self
            .unit
            .lookup(hash)
            .ok_or_else(|| VmError::from(VmErrorKind::MissingFunction { hash }))?;

        let args = info.signature.args;

        let (offset, call) = match &info.kind {
            UnitFnKind::Offset { offset, call } => (*offset, *call),
            _ => return Err(VmError::from(VmErrorKind::MissingFunction { hash })),
        };

        let environment = self.stack.pop_sequence(count)?;
        let environment = Shared::new(Tuple::from(environment));

        let fn_ptr = FnPtr::from_closure(
            self.context.clone(),
            self.unit.clone(),
            environment,
            offset,
            call,
            args,
        );
        self.stack.push(Value::FnPtr(Shared::new(fn_ptr)));
        Ok(())
    }

    /// Implementation of a function call.
    fn op_call(&mut self, hash: Hash, args: usize) -> Result<(), VmError> {
        match self.unit.lookup(hash) {
            Some(info) => {
                if info.signature.args != args {
                    return Err(VmError::from(VmErrorKind::ArgumentCountMismatch {
                        actual: args,
                        expected: info.signature.args,
                    }));
                }

                match info.kind {
                    UnitFnKind::Offset { offset, call } => {
                        self.call_offset_fn(offset, call, args)?;
                    }
                    UnitFnKind::Tuple { hash } => {
                        let tuple = self.stack.pop_sequence(info.signature.args)?;
                        let value = Value::typed_tuple(hash, tuple);
                        self.stack.push(value);
                    }
                    UnitFnKind::TupleVariant { enum_hash, hash } => {
                        let tuple = self.stack.pop_sequence(info.signature.args)?;
                        let value = Value::variant_tuple(enum_hash, hash, tuple);
                        self.stack.push(value);
                    }
                }
            }
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
        H: IntoTypeHash,
    {
        // NB: +1 to include the instance itself.
        let args = args + 1;
        let instance = self.stack.last()?;
        let value_type = instance.value_type()?;
        let hash = Hash::instance_function(value_type, hash);

        match self.unit.lookup(hash) {
            Some(info) => {
                if info.signature.args != args {
                    return Err(VmError::from(VmErrorKind::ArgumentCountMismatch {
                        actual: args,
                        expected: info.signature.args,
                    }));
                }

                match info.kind {
                    UnitFnKind::Offset { offset, call } => {
                        self.call_offset_fn(offset, call, args)?;
                    }
                    _ => {
                        return Err(VmError::from(VmErrorKind::MissingInstanceFunction {
                            instance: instance.type_info()?,
                            hash,
                        }));
                    }
                }
            }
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

    fn op_call_fn(&mut self, args: usize) -> Result<Option<StopReason>, VmError> {
        let function = self.stack.pop()?;

        let hash = match function {
            Value::Type(hash) => hash,
            Value::FnPtr(fn_ptr) => {
                let fn_ptr = fn_ptr.owned_ref()?;
                return fn_ptr.call_with_vm(self, args);
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
    pub(crate) async fn run_for(
        &mut self,
        mut limit: Option<usize>,
    ) -> Result<StopReason, VmError> {
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
                Inst::Add => {
                    self.op_add()?;
                }
                Inst::AddAssign { offset } => {
                    self.op_add_assign(offset)?;
                }
                Inst::Sub => {
                    self.op_sub()?;
                }
                Inst::SubAssign { offset } => {
                    self.op_sub_assign(offset)?;
                }
                Inst::Mul => {
                    self.op_mul()?;
                }
                Inst::MulAssign { offset } => {
                    self.op_mul_assign(offset)?;
                }
                Inst::Div => {
                    self.op_div()?;
                }
                Inst::DivAssign { offset } => {
                    self.op_div_assign(offset)?;
                }
                Inst::Rem => {
                    self.op_rem()?;
                }
                Inst::Fn { hash } => {
                    self.op_fn(hash)?;
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
                Inst::ObjectSlotIndexGet { slot } => {
                    self.op_object_slot_index_get(slot)?;
                }
                Inst::ObjectSlotIndexGetAt { offset, slot } => {
                    self.op_object_slot_index_get_at(offset, slot)?;
                }
                Inst::IndexSet => {
                    self.op_index_set()?;
                }
                Inst::Return => {
                    if self.op_return()? {
                        self.advance();
                        return Ok(StopReason::Exited);
                    }
                }
                Inst::ReturnUnit => {
                    if self.op_return_unit()? {
                        self.advance();
                        return Ok(StopReason::Exited);
                    }
                }
                Inst::Await => {
                    let future = self.op_await()?;
                    // NB: the future itself will advance the virtual machine.
                    return Ok(StopReason::Awaited(Awaited::Future(future)));
                }
                Inst::Select { len } => match self.op_select(len)? {
                    Some(select) => {
                        // NB: the future itself will advance the virtual machine.
                        return Ok(StopReason::Awaited(Awaited::Select(select)));
                    }
                    None => (),
                },
                Inst::Pop => {
                    self.stack.pop()?;
                }
                Inst::PopN { count } => {
                    self.op_popn(count)?;
                }
                Inst::PopAndJumpIf { count, offset } => {
                    self.op_pop_and_jump_if(count, offset)?;
                }
                Inst::PopAndJumpIfNot { count, offset } => {
                    self.op_pop_and_jump_if_not(count, offset)?;
                }
                Inst::Clean { count } => {
                    self.op_clean(count)?;
                }
                Inst::Integer { number } => {
                    self.stack.push(Value::Integer(number));
                }
                Inst::Float { number } => {
                    self.stack.push(Value::Float(number));
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
                Inst::Unit => {
                    self.stack.push(Value::Unit);
                }
                Inst::Bool { value } => {
                    self.stack.push(Value::Bool(value));
                }
                Inst::Vec { count } => {
                    self.op_vec(count)?;
                }
                Inst::Tuple { count } => {
                    self.op_tuple(count)?;
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
                Inst::Type { hash } => {
                    self.stack.push(Value::Type(hash));
                }
                Inst::Char { c } => {
                    self.stack.push(Value::Char(c));
                }
                Inst::Byte { b } => {
                    self.stack.push(Value::Byte(b));
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
                    return Ok(StopReason::Yielded);
                }
                Inst::YieldUnit => {
                    self.advance();
                    self.stack.push(Value::Unit);
                    return Ok(StopReason::Yielded);
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
                    return Ok(StopReason::Limited);
                }

                *limit -= 1;
            }
        }
    }
}

/// A call frame.
///
/// This is used to store the return point after an instruction has been run.
#[derive(Debug, Clone, Copy)]
struct CallFrame {
    /// The stored instruction pointer.
    ip: usize,
    /// The top of the stack at the time of the call to ensure stack isolation
    /// across function calls.
    ///
    /// I.e. a function should not be able to manipulate the size of any other
    /// stack than its own.
    stack_top: usize,
}

/// The reason why the virtual machine execution stopped.
#[derive(Debug)]
pub enum StopReason {
    /// The virtual machine exited by running out of call frames.
    Exited,
    /// The virtual machine exited because it ran out of execution quota.
    Limited,
    /// The virtual machine yielded.
    Yielded,
    /// The virtual machine awaited on the given future.
    Awaited(Awaited),
    /// Call into a new virtual machine.
    CallVm(CallVm),
}

impl StopReason {
    /// Convert into cheap info enum which only described the reason.
    pub fn into_info(self) -> StopReasonInfo {
        match self {
            Self::Exited => StopReasonInfo::Exited,
            Self::Limited => StopReasonInfo::Limited,
            Self::Yielded => StopReasonInfo::Yielded,
            Self::Awaited(..) => StopReasonInfo::Awaited,
            Self::CallVm(..) => StopReasonInfo::CallVm,
        }
    }
}

/// A stored await task.
#[derive(Debug)]
pub enum Awaited {
    /// A future to be awaited.
    Future(Shared<Future>),
    /// A select to be awaited.
    Select(Select),
}

impl Awaited {
    /// Wait for the given awaited into the specified virtual machine.
    pub(crate) async fn wait_with_vm(self, vm: &mut Vm) -> Result<(), VmError> {
        match self {
            Self::Future(future) => {
                let value = future.borrow_mut()?.await?;
                vm.stack.push(value);
                vm.advance();
            }
            Self::Select(select) => {
                let (branch, value) = select.await?;
                vm.stack.push(value);
                vm.stack.push(ToValue::to_value(branch)?);
                vm.advance();
            }
        }

        Ok(())
    }
}

/// An instruction to push a virtual machine to the execution.
#[derive(Debug)]
pub struct CallVm {
    pub(crate) call: UnitFnCall,
    pub(crate) vm: Vm,
}

impl CallVm {
    /// Construct a new nested vm call.
    pub(crate) fn new(call: UnitFnCall, vm: Vm) -> Self {
        Self { call, vm }
    }

    /// Encode the push itno an execution.
    pub(crate) fn into_execution<'vm>(self, execution: &mut VmExecution) -> Result<(), VmError> {
        match self.call {
            UnitFnCall::Generator => {
                let value = Value::from(Generator::new(self.vm));
                let vm = execution.vm_mut()?;
                vm.stack.push(value);
                vm.advance();
            }
            UnitFnCall::Immediate => {
                execution.push_vm(self.vm);
            }
            UnitFnCall::Async => {
                let future = Future::new(async move { self.vm.run_to_completion().await });
                let vm = execution.vm_mut()?;
                vm.stack.push(Value::from(future));
                vm.advance();
            }
        }

        Ok(())
    }
}

/// The reason why the virtual machine execution stopped.
#[derive(Debug, Clone, Copy)]
pub enum StopReasonInfo {
    /// The virtual machine exited by running out of call frames.
    Exited,
    /// The virtual machine exited because it ran out of execution quota.
    Limited,
    /// The virtual machine yielded.
    Yielded,
    /// The virtual machine awaited on the given future.
    Awaited,
    /// Received instruction to push the inner virtual machine.
    CallVm,
}

impl fmt::Display for StopReasonInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exited => write!(f, "exited"),
            Self::Limited => write!(f, "limited"),
            Self::Yielded => write!(f, "yielded"),
            Self::Awaited => write!(f, "awaited"),
            Self::CallVm => write!(f, "calling into other vm"),
        }
    }
}

/// The task of a unit being run.
pub struct Task<'a, T> {
    /// The virtual machine associated with the task.
    vm: &'a mut Vm,
    /// Marker holding output type.
    _marker: marker::PhantomData<&'a mut T>,
}

impl<'a, T> Task<'a, T>
where
    T: FromValue,
{
    /// Get access to the underlying virtual machine.
    pub fn vm(&self) -> &Vm {
        self.vm
    }

    /// Get access to the used compilation unit.
    pub fn unit(&self) -> &Unit {
        &*self.vm.unit
    }
}
