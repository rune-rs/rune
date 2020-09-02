use crate::context::Handler;
use crate::unit::UnitFnCall;
use crate::VmErrorKind;
use crate::{
    Context, FromValue, Future, Generator, Hash, IntoArgs, Shared, Stack, Tuple, Unit, Value, Vm,
    VmError,
};
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

/// A stored function, of some specific kind.
#[derive(Debug)]
pub struct FnPtr {
    inner: Inner,
}

impl FnPtr {
    /// Create a function pointer from a handler.
    pub fn from_handler(handler: Arc<Handler>) -> Self {
        Self {
            inner: Inner::FnHandler(FnHandler { handler }),
        }
    }

    /// Create a function pointer from an offset.
    pub fn from_offset(
        context: Rc<Context>,
        unit: Rc<Unit>,
        offset: usize,
        call: UnitFnCall,
        args: usize,
    ) -> Self {
        Self {
            inner: Inner::FnPtrOffset(FnPtrOffset {
                context,
                unit,
                offset,
                call,
                args,
            }),
        }
    }

    /// Create a function pointer from an offset.
    pub fn from_closure(
        context: Rc<Context>,
        unit: Rc<Unit>,
        environment: Shared<Tuple>,
        offset: usize,
        call: UnitFnCall,
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
    pub fn from_tuple(hash: Hash, args: usize) -> Self {
        Self {
            inner: Inner::FnTuple(FnTuple { hash, args }),
        }
    }

    /// Create a function pointer that constructs a tuple variant.
    pub fn from_variant_tuple(enum_hash: Hash, hash: Hash, args: usize) -> Self {
        Self {
            inner: Inner::FnVariantTuple(FnVariantTuple {
                enum_hash,
                hash,
                args,
            }),
        }
    }

    /// Perform a call over the function pointer.
    pub async fn call<A, T>(&self, args: A) -> Result<T, VmError>
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
            Inner::FnPtrOffset(offset) => {
                Self::check_args(A::count(), offset.args)?;

                let mut vm = Vm::new(offset.context.clone(), offset.unit.clone());
                vm.set_ip(offset.offset);
                args.into_args(vm.stack_mut())?;

                match offset.call {
                    UnitFnCall::Generator => Value::Generator(Shared::new(Generator::new(vm))),
                    UnitFnCall::Immediate => {
                        Future::new(async move { vm.run().run_to_completion().await }).await?
                    }
                    UnitFnCall::Async => Value::Future(Shared::new(Future::new(async move {
                        vm.run().run_to_completion().await
                    }))),
                }
            }
            Inner::FnClosureOffset(offset) => {
                Self::check_args(A::count(), offset.args)?;

                let mut vm = Vm::new(offset.context.clone(), offset.unit.clone());
                vm.set_ip(offset.offset);
                args.into_args(vm.stack_mut())?;
                vm.stack_mut()
                    .push(Value::Tuple(offset.environment.clone()));

                Self::call_vm(offset.call, vm).await?
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

    /// Call with the given stack.
    pub(crate) async fn call_with_vm(&self, vm: &mut Vm, args: usize) -> Result<(), VmError> {
        let value = match &self.inner {
            Inner::FnHandler(handler) => {
                return Ok((handler.handler)(vm.stack_mut(), args)?);
            }
            Inner::FnPtrOffset(offset) => {
                Self::check_args(args, offset.args)?;

                // Fast past, just allocate a call frame and keep running.
                if let UnitFnCall::Immediate = offset.call {
                    if vm.is_same(&offset.context, &offset.unit) {
                        vm.push_call_frame(offset.offset, args)?;
                        return Ok(());
                    }
                }

                let new_stack = vm.stack_mut().drain_stack_top(args)?.collect::<Stack>();
                let mut vm =
                    Vm::new_with_stack(offset.context.clone(), offset.unit.clone(), new_stack);
                vm.set_ip(offset.offset);

                Self::call_vm(offset.call, vm).await?
            }
            Inner::FnClosureOffset(offset) => {
                Self::check_args(args, offset.args)?;

                // Fast past, just allocate a call frame, push the environment
                // onto the stack and keep running.
                if let UnitFnCall::Immediate = offset.call {
                    if vm.is_same(&offset.context, &offset.unit) {
                        vm.push_call_frame(offset.offset, args)?;
                        vm.stack_mut()
                            .push(Value::Tuple(offset.environment.clone()));
                        return Ok(());
                    }
                }

                let mut new_stack = Stack::new();
                new_stack.extend(vm.stack_mut().drain_stack_top(args)?);
                new_stack.push(Value::Tuple(offset.environment.clone()));
                let mut vm =
                    Vm::new_with_stack(offset.context.clone(), offset.unit.clone(), new_stack);
                vm.set_ip(offset.offset);

                Self::call_vm(offset.call, vm).await?
            }
            Inner::FnTuple(tuple) => {
                Self::check_args(args, tuple.args)?;
                Value::typed_tuple(tuple.hash, vm.stack_mut().pop_sequence(args)?)
            }
            Inner::FnVariantTuple(tuple) => {
                Self::check_args(args, tuple.args)?;
                Value::variant_tuple(
                    tuple.enum_hash,
                    tuple.hash,
                    vm.stack_mut().pop_sequence(args)?,
                )
            }
        };

        vm.stack_mut().push(value);
        Ok(())
    }

    #[inline]
    fn check_args(actual: usize, expected: usize) -> Result<(), VmError> {
        if actual != expected {
            return Err(VmError::from(VmErrorKind::ArgumentCountMismatch {
                expected,
                actual,
            }));
        }

        Ok(())
    }

    #[inline]
    async fn call_vm(call: UnitFnCall, mut vm: Vm) -> Result<Value, VmError> {
        match call {
            UnitFnCall::Generator => Ok(Value::Generator(Shared::new(Generator::new(vm)))),
            UnitFnCall::Immediate => {
                let future = Future::new(async move { vm.run().run_to_completion().await });
                Ok(future.await?)
            }
            UnitFnCall::Async => {
                let future = Future::new(async move { vm.run().run_to_completion().await });
                Ok(Value::Future(Shared::new(future)))
            }
        }
    }
}

#[derive(Debug)]
enum Inner {
    FnHandler(FnHandler),
    FnPtrOffset(FnPtrOffset),
    FnTuple(FnTuple),
    FnClosureOffset(FnClosureOffset),
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

struct FnPtrOffset {
    context: Rc<Context>,
    /// The unit where the function resides.
    unit: Rc<Unit>,
    /// The offset of the function.
    offset: usize,
    /// The calling convention.
    call: UnitFnCall,
    /// The number of arguments the function takes.
    args: usize,
}

impl fmt::Debug for FnPtrOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FnPtrOffset")
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
    call: UnitFnCall,
    /// The number of arguments the function takes.
    args: usize,
}

impl fmt::Debug for FnClosureOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FnPtrOffset")
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
