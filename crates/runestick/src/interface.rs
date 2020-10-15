use crate::{
    Args, Context, FromValue, Hash, IntoTypeHash, Iterator, Stack, Unit, UnitFn, Value, Vm,
    VmError, VmErrorKind,
};
use std::cell::Cell;
use std::marker;
use std::ptr;
use std::sync::Arc;

thread_local! { static ENV: Cell<Env> = Cell::new(Env::null()) }

/// An interface which wraps a value and allows for accessing protocols.
///
/// This can be used as an argument type for native functions who wants to call
/// a protocol function like [INTO_ITER](crate::INTO_ITER) (see
/// [into_iter][Self::into_iter]).
pub struct Interface {
    target: Value,
    unit: Arc<Unit>,
    context: Arc<Context>,
}

impl Interface {
    /// Call the `into_iter` protocol on the value.
    pub fn into_iter(mut self) -> Result<Iterator, VmError> {
        let target = match std::mem::take(&mut self.target) {
            Value::Iterator(iterator) => return Ok(iterator.take()?),
            Value::Vec(vec) => return Ok(vec.borrow_ref()?.into_iterator()),
            Value::Object(object) => return Ok(object.borrow_ref()?.into_iterator()),
            target => target,
        };

        let value = self.call_instance_fn(crate::INTO_ITER, target, ())?;
        Iterator::from_value(value)
    }

    /// Helper function to call an instance function.
    fn call_instance_fn<H, A>(self, hash: H, target: Value, args: A) -> Result<Value, VmError>
    where
        H: IntoTypeHash,
        A: Args,
    {
        let count = args.count() + 1;
        let hash = Hash::instance_function(target.type_of()?, hash.into_type_hash());

        if let Some(UnitFn::Offset {
            offset,
            args: expected,
            call,
        }) = self.unit.lookup(hash)
        {
            Self::check_args(count, expected)?;

            let mut stack = Stack::with_capacity(count);
            stack.push(target);
            args.into_stack(&mut stack)?;

            let mut vm = Vm::new_with_stack(self.context.clone(), self.unit.clone(), stack);
            vm.set_ip(offset);
            return call.call_with_vm(vm);
        }

        let handler = match self.context.lookup(hash) {
            Some(handler) => handler,
            None => return Err(VmError::from(VmErrorKind::MissingFunction { hash })),
        };

        let mut stack = Stack::with_capacity(count);
        stack.push(target);
        args.into_stack(&mut stack)?;

        handler(&mut stack, count)?;
        Ok(stack.pop()?)
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

impl FromValue for Interface {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let env = ENV.with(|env| env.get());
        let Env { context, unit } = env;

        if context.is_null() || unit.is_null() {
            return Err(VmError::from(VmErrorKind::MissingInterfaceEnvironment));
        }

        // Safety: context and unit can only be registered publicly through
        // [EnvGuard], which makes sure that they are live for the duration of
        // the registration.
        Ok(Interface {
            target: value,
            context: unsafe { (*context).clone() },
            unit: unsafe { (*unit).clone() },
        })
    }
}

pub(crate) struct EnvGuard<'a> {
    old: Env,
    _marker: marker::PhantomData<&'a ()>,
}

impl<'a> EnvGuard<'a> {
    /// Construct a new environment guard with the given context and unit.
    pub(crate) fn new(context: &'a Arc<Context>, unit: &'a Arc<Unit>) -> EnvGuard<'a> {
        let old = ENV.with(|e| e.replace(Env { context, unit }));

        EnvGuard {
            old,
            _marker: marker::PhantomData,
        }
    }
}

impl Drop for EnvGuard<'_> {
    fn drop(&mut self) {
        ENV.with(|e| e.set(self.old));
    }
}

#[derive(Debug, Clone, Copy)]
struct Env {
    context: *const Arc<Context>,
    unit: *const Arc<Unit>,
}

impl Env {
    const fn null() -> Self {
        Self {
            context: ptr::null(),
            unit: ptr::null(),
        }
    }
}
