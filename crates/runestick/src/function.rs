use crate::context::Handler;
use crate::VmErrorKind;
use crate::{
    Args, Call, Context, FromValue, Future, Generator, Hash, RawRef, Ref, Shared, Stack, Stream,
    Tuple, Unit, UnsafeFromValue, Value, Vm, VmCall, VmError, VmHalt,
};
use std::fmt;
use std::sync::Arc;

/// A stored function, of some specific kind.
pub struct Function {
    inner: Inner,
}

impl Function {
    /// Perform a call over the function represented by this function pointer.
    pub fn call<A, T>(&self, args: A) -> Result<T, VmError>
    where
        A: Args,
        T: FromValue,
    {
        let value = match &self.inner {
            Inner::FnHandler(handler) => {
                let mut stack = Stack::with_capacity(A::count());
                args.into_stack(&mut stack)?;
                (handler.handler)(&mut stack, A::count())?;
                stack.pop()?
            }
            Inner::FnOffset(fn_offset) => fn_offset.call(args, ())?,
            Inner::FnClosureOffset(closure) => closure
                .fn_offset
                .call(args, (closure.environment.clone(),))?,
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

    /// Call with the given virtual machine. This allows for certain
    /// optimizations, like avoiding the allocation of a new vm state in case
    /// the call is internal.
    ///
    /// A stop reason will be returned in case the function call results in
    /// a need to suspend the execution.
    pub(crate) fn call_with_vm(&self, vm: &mut Vm, args: usize) -> Result<Option<VmHalt>, VmError> {
        let reason = match &self.inner {
            Inner::FnHandler(handler) => {
                (handler.handler)(vm.stack_mut(), args)?;
                None
            }
            Inner::FnOffset(fn_offset) => {
                if let Some(vm_call) = fn_offset.call_with_vm(vm, args, ())? {
                    return Ok(Some(VmHalt::VmCall(vm_call)));
                }

                None
            }
            Inner::FnClosureOffset(closure) => {
                if let Some(vm_call) =
                    closure
                        .fn_offset
                        .call_with_vm(vm, args, (closure.environment.clone(),))?
                {
                    return Ok(Some(VmHalt::VmCall(vm_call)));
                }

                None
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

    /// Create a function pointer from a handler.
    pub(crate) fn from_handler(handler: Arc<Handler>) -> Self {
        Self {
            inner: Inner::FnHandler(FnHandler { handler }),
        }
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_offset(
        context: Arc<Context>,
        unit: Arc<Unit>,
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
        context: Arc<Context>,
        unit: Arc<Unit>,
        offset: usize,
        call: Call,
        args: usize,
        environment: Shared<Tuple>,
    ) -> Self {
        Self {
            inner: Inner::FnClosureOffset(FnClosureOffset {
                fn_offset: FnOffset {
                    context,
                    unit,
                    offset,
                    call,
                    args,
                },
                environment,
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
                    closure.fn_offset.offset, closure.environment
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
    /// A native function handler.
    /// This is wrapped as an `Arc<dyn Handler>`.
    FnHandler(FnHandler),
    /// The offset to a free function.
    ///
    /// This also captures the context and unit it belongs to allow for external
    /// calls.
    FnOffset(FnOffset),
    /// A closure with a captured environment.
    ///
    /// This also captures the context and unit it belongs to allow for external
    /// calls.
    FnClosureOffset(FnClosureOffset),
    /// Constructor for a tuple.
    FnTuple(FnTuple),
    /// Constructor for a tuple variant.
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
    context: Arc<Context>,
    /// The unit where the function resides.
    unit: Arc<Unit>,
    /// The offset of the function.
    offset: usize,
    /// The calling convention.
    call: Call,
    /// The number of arguments the function takes.
    args: usize,
}

impl FnOffset {
    /// Perform a call into the specified offset and return the produced value.
    fn call<A, E>(&self, args: A, extra: E) -> Result<Value, VmError>
    where
        A: Args,
        E: Args,
    {
        Function::check_args(A::count(), self.args)?;

        let mut vm = Vm::new(self.context.clone(), self.unit.clone());

        vm.set_ip(self.offset);
        args.into_stack(vm.stack_mut())?;
        extra.into_stack(vm.stack_mut())?;

        Ok(match self.call {
            Call::Stream => Value::from(Stream::new(vm)),
            Call::Generator => Value::from(Generator::new(vm)),
            Call::Immediate => vm.complete()?,
            Call::Async => Value::from(Future::new(vm.async_complete())),
        })
    }

    /// Perform a potentially optimized call into the specified vm.
    ///
    /// This will cause a halt in case the vm being called into isn't the same
    /// as the context and unit of the function.
    fn call_with_vm<E>(&self, vm: &mut Vm, args: usize, extra: E) -> Result<Option<VmCall>, VmError>
    where
        E: Args,
    {
        Function::check_args(args, self.args)?;

        // Fast past, just allocate a call frame and keep running.
        if let Call::Immediate = self.call {
            if vm.is_same(&self.context, &self.unit) {
                vm.push_call_frame(self.offset, args)?;
                extra.into_stack(vm.stack_mut())?;
                return Ok(None);
            }
        }

        let mut new_stack = vm.stack_mut().drain_stack_top(args)?.collect::<Stack>();
        extra.into_stack(&mut new_stack)?;
        let mut vm = Vm::new_with_stack(self.context.clone(), self.unit.clone(), new_stack);
        vm.set_ip(self.offset);
        Ok(Some(VmCall::new(self.call, vm)))
    }
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

#[derive(Debug)]
struct FnClosureOffset {
    /// Function offset.
    fn_offset: FnOffset,
    /// Captured environment.
    environment: Shared<Tuple>,
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

impl FromValue for Ref<Function> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_function()?.into_ref()?)
    }
}

impl UnsafeFromValue for &Function {
    type Output = *const Function;
    type Guard = RawRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let function = value.into_function()?;
        let (function, guard) = Ref::into_raw(function.into_ref()?);
        Ok((function, guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}
