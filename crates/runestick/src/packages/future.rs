//! The future package.

use crate::future::SelectFuture;
use crate::{ContextError, Future, Module, OwnedMut, Shared, Stack, Value, VmError};

async fn try_join_impl<'a, I, F>(values: I, len: usize, factory: F) -> Result<Value, VmError>
where
    I: IntoIterator<Item = &'a Value>,
    F: FnOnce(Vec<Value>) -> Value,
{
    use futures::StreamExt as _;

    let mut futures = futures::stream::FuturesUnordered::new();
    let mut guards = Vec::with_capacity(len);
    let mut results = Vec::with_capacity(len);

    for (index, value) in values.into_iter().enumerate() {
        let future = match value {
            Value::Future(future) => future.clone().owned_mut()?,
            value => {
                return Err(VmError::BadArgument {
                    argument: value.type_info()?,
                })
            }
        };

        let (raw_future, guard) = OwnedMut::into_raw(future);

        // Safety: since we are in the virtual machine, anything the future
        // references can be safely used.
        unsafe {
            futures.push(SelectFuture::new_unchecked(raw_future, index));
            guards.push(guard);
            results.push(Value::Unit);
        }
    }

    while !futures.is_empty() {
        let (index, value) = futures.next().await.unwrap()?;
        *results.get_mut(index).unwrap() = value;
    }

    Ok(factory(results))
}

async fn join(value: Value) -> Result<Value, VmError> {
    match value {
        Value::Tuple(tuple) => {
            let tuple = tuple.borrow_ref()?;
            Ok(try_join_impl(tuple.iter(), tuple.len(), Value::tuple).await?)
        }
        Value::Vec(vec) => {
            let vec = vec.borrow_ref()?;
            Ok(try_join_impl(vec.iter(), vec.len(), Value::vec).await?)
        }
        value => Err(VmError::BadArgument {
            argument: value.type_info()?,
        }),
    }
}

/// The join implementation.
fn raw_join(stack: &mut Stack, args: usize) -> Result<(), VmError> {
    if args != 1 {
        return Err(VmError::ArgumentCountMismatch {
            actual: args,
            expected: 1,
        });
    }

    let value = stack.pop()?;
    let value = Value::Future(Shared::new(Future::new(join(value))));
    stack.push(value);
    Ok(())
}

/// Get the module for the future package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "future"]);
    module.raw_fn(&["join"], raw_join)?;
    Ok(module)
}
