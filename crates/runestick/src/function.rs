use crate::context::Handler;
use crate::VmErrorKind;
use crate::{
    Call, CallVm, Context, FromValue, Future, Generator, Hash, IntoArgs, OwnedRef, RawOwnedRef,
    Shared, Stack, StopReason, Stream, Tuple, Unit, UnsafeFromValue, Value, Vm, VmError,
};
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

value_types!(crate::FUNCTION_TYPE, Function => Function, &Function, Shared<Function>, OwnedRef<Function>);

/// A stored function, of some specific kind.
pub struct Function {
    inner: Inner,
}

impl Function {
    /// Perform a call over the function represented by this function pointer.
    pub fn call<A, T>(&self, args: A) -> Result<T, VmError>
    where
        A: IntoArgs,
        T: FromValue,
    {
        let value = match &self.inner {
            Inner::FnHandler(handler) => {
                let mut stack = Stack::with_capacity(A::count());
                args.into_args(&mut stack)?;
                (handler.handler)(&mut stack, A::count())?;
                stack.pop()?
            }
            Inner::FnOffset(offset) => {
                Self::check_args(A::count(), offset.args)?;

                let mut vm = Vm::new(offset.context.clone(), offset.unit.clone());
                vm.set_ip(offset.offset);
                args.into_args(vm.stack_mut())?;

                match offset.call {
                    Call::Stream => Value::from(Stream::new(vm)),
                    Call::Generator => Value::from(Generator::new(vm)),
                    Call::Immediate => vm.complete()?,
                    Call::Async => Value::from(Future::new(vm.async_complete())),
                }
            }
            Inner::FnClosureOffset(closure) => {
                Self::check_args(A::count(), closure.args)?;

                let mut vm = Vm::new(closure.context.clone(), closure.unit.clone());
                vm.set_ip(closure.offset);
                args.into_args(vm.stack_mut())?;
                vm.stack_mut().push(closure.environment.clone());

                match closure.call {
                    Call::Stream => Value::from(Stream::new(vm)),
                    Call::Generator => Value::from(Generator::new(vm)),
                    Call::Immediate => vm.complete()?,
                    Call::Async => Value::from(Future::new(vm.async_complete())),
                }
            }
            Inner::FnTuple(tuple) => {
                Self::check_args(A::count(), tuple.args)?;

                Value::typed_tuple(tuple.hash, args.into_vec()?)
            }
            Inner::FnVariantTuple(tuple) => {
                Self::check_args(A::count(), tuple.args)?;

                Value::variant_tuple(tuple.enum_hash, tuple.hash, args.into_vec()?)
            }
        };

        Ok(T::from_value(value)?)
    }

    /// Create a function pointer from a handler.
    pub(crate) fn from_handler(handler: Arc<Handler>) -> Self {
        Self {
            inner: Inner::FnHandler(FnHandler { handler }),
        }
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_offset(
        context: Rc<Context>,
        unit: Rc<Unit>,
        offset: usize,
        call: Call,
        args: usize,
    ) -> Self {
        Self {
            inner: Inner::FnOffset(FnOffset {
                context,
                unit,
                offset,
                call,
                args,
            }),
        }
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_closure(
        context: Rc<Context>,
        unit: Rc<Unit>,
        environment: Shared<Tuple>,
        offset: usize,
        call: Call,
        args: usize,
    ) -> Self {
        Self {
            inner: Inner::FnClosureOffset(FnClosureOffset {
                context,
                unit,
                environment,
                offset,
                call,
                args,
            }),
        }
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_tuple(hash: Hash, args: usize) -> Self {
        Self {
            inner: Inner::FnTuple(FnTuple { hash, args }),
        }
    }

    /// Create a function pointer that constructs a tuple variant.
    pub(crate) fn from_variant_tuple(enum_hash: Hash, hash: Hash, args: usize) -> Self {
        Self {
            inner: Inner::FnVariantTuple(FnVariantTuple {
                enum_hash,
                hash,
                args,
            }),
        }
    }

    /// Call with the given virtual machine. This allows for certain
    /// optimizations, like avoiding the allocation of a new vm state in case
    /// the call is internal.
    ///
    /// A stop reason will be returned in case the function call results in
    /// a need to suspend the execution.
    pub(crate) fn call_with_vm(
        &self,
        vm: &mut Vm,
        args: usize,
    ) -> Result<Option<StopReason>, VmError> {
        let reason = match &self.inner {
            Inner::FnHandler(handler) => {
                (handler.handler)(vm.stack_mut(), args)?;
                None
            }
            Inner::FnOffset(offset) => {
                Self::check_args(args, offset.args)?;

                // Fast past, just allocate a call frame and keep running.
                if let Call::Immediate = offset.call {
                    if vm.is_same(&offset.context, &offset.unit) {
                        vm.push_call_frame(offset.offset, args)?;
                        return Ok(None);
                    }
                }

                let new_stack = vm.stack_mut().drain_stack_top(args)?.collect::<Stack>();
                let mut vm =
                    Vm::new_with_stack(offset.context.clone(), offset.unit.clone(), new_stack);
                vm.set_ip(offset.offset);
                Some(StopReason::CallVm(CallVm::new(offset.call, vm)))
            }
            Inner::FnClosureOffset(offset) => {
                Self::check_args(args, offset.args)?;

                // Fast past, just allocate a call frame, push the environment
                // onto the stack and keep running.
                if let Call::Immediate = offset.call {
                    if vm.is_same(&offset.context, &offset.unit) {
                        vm.push_call_frame(offset.offset, args)?;
                        vm.stack_mut()
                            .push(Value::Tuple(offset.environment.clone()));
                        return Ok(None);
                    }
                }

                let mut new_stack = Stack::new();
                new_stack.extend(vm.stack_mut().drain_stack_top(args)?);
                new_stack.push(Value::Tuple(offset.environment.clone()));
                let mut vm =
                    Vm::new_with_stack(offset.context.clone(), offset.unit.clone(), new_stack);
                vm.set_ip(offset.offset);
                Some(StopReason::CallVm(CallVm::new(offset.call, vm)))
            }
            Inner::FnTuple(tuple) => {
                Self::check_args(args, tuple.args)?;

                let value = Value::typed_tuple(tuple.hash, vm.stack_mut().pop_sequence(args)?);
                vm.stack_mut().push(value);
                None
            }
            Inner::FnVariantTuple(tuple) => {
                Self::check_args(args, tuple.args)?;

                let value = Value::variant_tuple(
                    tuple.enum_hash,
                    tuple.hash,
                    vm.stack_mut().pop_sequence(args)?,
                );

                vm.stack_mut().push(value);
                None
            }
        };

        Ok(reason)
    }

    #[inline]
    fn check_args(actual: usize, expected: usize) -> Result<(), VmError> {
        if actual != expected {
            return Err(VmError::from(VmErrorKind::BadArgumentCount {
                expected,
                actual,
            }));
        }

        Ok(())
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            Inner::FnHandler(handler) => {
                write!(f, "native function ({:p})", handler.handler.as_ref())?;
            }
            Inner::FnOffset(offset) => {
                write!(f, "dynamic function (at: 0x{:x})", offset.offset)?;
            }
            Inner::FnClosureOffset(closure) => {
                write!(
                    f,
                    "closure (at: 0x{:x}, env:{:?})",
                    closure.offset, closure.environment
                )?;
            }
            Inner::FnTuple(tuple) => {
                write!(f, "tuple (type: {})", tuple.hash)?;
            }
            Inner::FnVariantTuple(tuple) => {
                write!(
                    f,
                    "variant tuple (enum: {}, type: {})",
                    tuple.enum_hash, tuple.hash
                )?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
enum Inner {
    FnHandler(FnHandler),
    FnOffset(FnOffset),
    FnClosureOffset(FnClosureOffset),
    FnTuple(FnTuple),
    FnVariantTuple(FnVariantTuple),
}

struct FnHandler {
    /// The function handler.
    handler: Arc<Handler>,
}

impl fmt::Debug for FnHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FnHandler")
    }
}

struct FnOffset {
    context: Rc<Context>,
    /// The unit where the function resides.
    unit: Rc<Unit>,
    /// The offset of the function.
    offset: usize,
    /// The calling convention.
    call: Call,
    /// The number of arguments the function takes.
    args: usize,
}

impl fmt::Debug for FnOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FnOffset")
            .field("context", &(&self.context as *const _))
            .field("unit", &(&self.unit as *const _))
            .field("offset", &self.offset)
            .field("call", &self.call)
            .field("args", &self.args)
            .finish()
    }
}

struct FnClosureOffset {
    context: Rc<Context>,
    /// The unit where the function resides.
    unit: Rc<Unit>,
    /// Captured environment.
    environment: Shared<Tuple>,
    /// The offset of the function.
    offset: usize,
    /// The calling convention.
    call: Call,
    /// The number of arguments the function takes.
    args: usize,
}

impl fmt::Debug for FnClosureOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FnOffset")
            .field("context", &(&self.context as *const _))
            .field("unit", &(&self.unit as *const _))
            .field("environment", &self.environment)
            .field("offset", &self.offset)
            .field("call", &self.call)
            .field("args", &self.args)
            .finish()
    }
}

#[derive(Debug)]
struct FnTuple {
    /// The type of the tuple.
    hash: Hash,
    /// The number of arguments the tuple takes.
    args: usize,
}

#[derive(Debug)]
struct FnVariantTuple {
    /// The enum the variant belongs to.
    enum_hash: Hash,
    /// The type of the tuple.
    hash: Hash,
    /// The number of arguments the tuple takes.
    args: usize,
}

impl FromValue for Function {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_function()?.take()?)
    }
}

impl FromValue for Shared<Function> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_function()?)
    }
}

impl FromValue for OwnedRef<Function> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_function()?.owned_ref()?)
    }
}

impl UnsafeFromValue for &Function {
    type Output = *const Function;
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let function = value.into_function()?;
        let (function, guard) = OwnedRef::into_raw(function.owned_ref()?);
        Ok((function, guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}
