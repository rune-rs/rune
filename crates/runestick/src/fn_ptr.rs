use crate::context::Handler;
use crate::unit::UnitFnCall;
use crate::{
    CompilationUnit, Context, FromValue, Future, Hash, IntoArgs, Shared, Stack, Value, Vm, VmError,
};
use std::fmt;
use std::rc::Rc;

/// A stored function, of some specific kind.
#[derive(Debug)]
pub struct FnPtr {
    inner: Inner,
}

impl FnPtr {
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
                if A::count() != offset.args {
                    return Err(VmError::ArgumentCountMismatch {
                        expected: offset.args,
                        actual: A::count(),
                    });
                }

                let mut vm = Vm::new(offset.context.clone(), offset.unit.clone());
                vm.set_ip(offset.offset);
                args.into_args(vm.stack_mut())?;

                match offset.call {
                    UnitFnCall::Immediate => vm.run().run_to_completion().await?,
                    UnitFnCall::Async => Value::Future(Shared::new(Future::new(async move {
                        Ok(vm.run().run_to_completion().await?)
                    }))),
                }
            }
            Inner::FnTuple(tuple) => {
                if A::count() != tuple.args {
                    return Err(VmError::ArgumentCountMismatch {
                        expected: tuple.args,
                        actual: A::count(),
                    });
                }

                Value::typed_tuple(tuple.hash, args.into_vec()?)
            }
            Inner::FnVariantTuple(tuple) => {
                if A::count() != tuple.args {
                    return Err(VmError::ArgumentCountMismatch {
                        expected: tuple.args,
                        actual: A::count(),
                    });
                }

                Value::variant_tuple(tuple.enum_hash, tuple.hash, args.into_vec()?)
            }
        };

        Ok(T::from_value(value)?)
    }

    /// Call with the given stack.
    pub async fn call_with_stack(&self, stack: &mut Stack, args: usize) -> Result<(), VmError> {
        let value = match &self.inner {
            Inner::FnHandler(handler) => {
                return Ok((handler.handler)(stack, args)?);
            }
            Inner::FnPtrOffset(offset) => {
                if args != offset.args {
                    return Err(VmError::ArgumentCountMismatch {
                        expected: offset.args,
                        actual: args,
                    });
                }

                let new_stack = stack.pop_sub_stack(args)?;
                let mut vm =
                    Vm::new_with_stack(offset.context.clone(), offset.unit.clone(), new_stack);
                vm.set_ip(offset.offset);

                let future = Future::new(async move { Ok(vm.run().run_to_completion().await?) });

                match offset.call {
                    UnitFnCall::Immediate => future.await?,
                    UnitFnCall::Async => Value::Future(Shared::new(future)),
                }
            }
            Inner::FnTuple(tuple) => {
                if args != tuple.args {
                    return Err(VmError::ArgumentCountMismatch {
                        expected: tuple.args,
                        actual: args,
                    });
                }

                Value::typed_tuple(tuple.hash, stack.pop_sequence(args)?)
            }
            Inner::FnVariantTuple(tuple) => {
                if args != tuple.args {
                    return Err(VmError::ArgumentCountMismatch {
                        expected: tuple.args,
                        actual: args,
                    });
                }

                Value::variant_tuple(tuple.enum_hash, tuple.hash, stack.pop_sequence(args)?)
            }
        };

        stack.push(value);
        Ok(())
    }

    /// Create a function pointer from a handler.
    pub fn from_handler(handler: Rc<Handler>) -> Self {
        Self {
            inner: Inner::FnHandler(FnHandler { handler }),
        }
    }

    /// Create a function pointer from an offset.
    pub fn from_offset(
        context: Rc<Context>,
        unit: Rc<CompilationUnit>,
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
}

#[derive(Debug)]
enum Inner {
    FnHandler(FnHandler),
    FnPtrOffset(FnPtrOffset),
    FnTuple(FnTuple),
    FnVariantTuple(FnVariantTuple),
}

struct FnHandler {
    /// The function handler.
    handler: Rc<Handler>,
}

impl fmt::Debug for FnHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FnHandler")
    }
}

struct FnPtrOffset {
    context: Rc<Context>,
    /// The unit where the function resides.
    unit: Rc<CompilationUnit>,
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
