use crate::context::Handler;
use crate::VmErrorKind;
use crate::{
    Args, Call, Context, FromValue, Future, Generator, RawRef, Ref, Rtti, Shared, Stack, Stream,
    Tuple, Unit, UnsafeFromValue, Value, VariantRtti, Vm, VmCall, VmError, VmHalt,
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
                let arg_count = args.count();
                let mut stack = Stack::with_capacity(arg_count);
                args.into_stack(&mut stack)?;
                (handler.handler)(&mut stack, arg_count)?;
                stack.pop()?
            }
            Inner::FnOffset(fn_offset) => fn_offset.call(args, ())?,
            Inner::FnClosureOffset(closure) => closure
                .fn_offset
                .call(args, (closure.environment.clone(),))?,
            Inner::FnUnitStruct(empty) => {
                Self::check_args(args.count(), 0)?;
                Value::unit_struct(empty.rtti.clone())
            }
            Inner::FnTupleStruct(tuple) => {
                Self::check_args(args.count(), tuple.args)?;
                Value::tuple_struct(tuple.rtti.clone(), args.into_vec()?)
            }
            Inner::FnUnitVariant(empty) => {
                Self::check_args(args.count(), 0)?;
                Value::empty_variant(empty.rtti.clone())
            }
            Inner::FnTupleVariant(tuple) => {
                Self::check_args(args.count(), tuple.args)?;
                Value::tuple_variant(tuple.rtti.clone(), args.into_vec()?)
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
            Inner::FnUnitStruct(empty) => {
                Self::check_args(args, 0)?;
                vm.stack_mut().push(Value::unit_struct(empty.rtti.clone()));
                None
            }
            Inner::FnTupleStruct(tuple) => {
                Self::check_args(args, tuple.args)?;

                let value =
                    Value::tuple_struct(tuple.rtti.clone(), vm.stack_mut().pop_sequence(args)?);
                vm.stack_mut().push(value);
                None
            }
            Inner::FnUnitVariant(tuple) => {
                Self::check_args(args, 0)?;

                let value = Value::empty_variant(tuple.rtti.clone());
                vm.stack_mut().push(value);
                None
            }
            Inner::FnTupleVariant(tuple) => {
                Self::check_args(args, tuple.args)?;

                let value =
                    Value::tuple_variant(tuple.rtti.clone(), vm.stack_mut().pop_sequence(args)?);
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
    pub(crate) fn from_unit_struct(rtti: Arc<Rtti>) -> Self {
        Self {
            inner: Inner::FnUnitStruct(FnUnitStruct { rtti }),
        }
    }

    /// Create a function pointer from an offset.
    pub(crate) fn from_tuple_struct(rtti: Arc<Rtti>, args: usize) -> Self {
        Self {
            inner: Inner::FnTupleStruct(FnTupleStruct { rtti, args }),
        }
    }

    /// Create a function pointer that constructs a empty variant.
    pub(crate) fn from_empty_variant(rtti: Arc<VariantRtti>) -> Self {
        Self {
            inner: Inner::FnUnitVariant(FnUnitVariant { rtti }),
        }
    }

    /// Create a function pointer that constructs a tuple variant.
    pub(crate) fn from_tuple_variant(rtti: Arc<VariantRtti>, args: usize) -> Self {
        Self {
            inner: Inner::FnTupleVariant(FnTupleVariant { rtti, args }),
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
            Inner::FnUnitStruct(empty) => {
                write!(f, "empty {}", empty.rtti.item)?;
            }
            Inner::FnTupleStruct(tuple) => {
                write!(f, "tuple {}", tuple.rtti.item)?;
            }
            Inner::FnUnitVariant(empty) => {
                write!(f, "variant empty {}", empty.rtti.item)?;
            }
            Inner::FnTupleVariant(tuple) => {
                write!(f, "variant tuple {}", tuple.rtti.item)?;
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
    /// Constructor for a unit struct.
    FnUnitStruct(FnUnitStruct),
    /// Constructor for a tuple.
    FnTupleStruct(FnTupleStruct),
    /// Constructor for an empty variant.
    FnUnitVariant(FnUnitVariant),
    /// Constructor for a tuple variant.
    FnTupleVariant(FnTupleVariant),
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
        Function::check_args(args.count(), self.args)?;

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
struct FnUnitStruct {
    /// The type of the empty.
    rtti: Arc<Rtti>,
}

#[derive(Debug)]
struct FnTupleStruct {
    /// The type of the tuple.
    rtti: Arc<Rtti>,
    /// The number of arguments the tuple takes.
    args: usize,
}

#[derive(Debug)]
struct FnUnitVariant {
    /// Runtime information fo variant.
    rtti: Arc<VariantRtti>,
}

#[derive(Debug)]
struct FnTupleVariant {
    /// Runtime information fo variant.
    rtti: Arc<VariantRtti>,
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

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let function = value.into_function()?;
        let (function, guard) = Ref::into_raw(function.into_ref()?);
        Ok((function, guard))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}
