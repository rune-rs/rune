//! The `std::future` module.

use crate::future::SelectFuture;
use crate::{ContextError, Future, Module, Shared, Stack, Value, VmError, VmErrorKind};

/// Construct the `std::future` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "future"]);
    module.ty::<Future>()?;
    module.raw_fn(&["join"], raw_join)?;
    Ok(module)
}

async fn try_join_impl<'a, I, F>(values: I, len: usize, factory: F) -> Result<Value, VmError>
where
    I: IntoIterator<Item = &'a Value>,
    F: FnOnce(Vec<Value>) -> Value,
{
    use futures_util::stream::StreamExt as _;

    let mut futures = futures_util::stream::FuturesUnordered::new();
    let mut results = Vec::with_capacity(len);

    for (index, value) in values.into_iter().enumerate() {
        let future = match value {
            Value::Future(future) => future.clone().into_mut()?,
            value => return Err(VmError::bad_argument::<Future>(index, value)?),
        };

        futures.push(SelectFuture::new(index, future));
        results.push(Value::Unit);
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
        value => Err(VmError::bad_argument::<Vec<Value>>(0, &value)?),
    }
}

/// The join implementation.
fn raw_join(stack: &mut Stack, args: usize) -> Result<(), VmError> {
    if args != 1 {
        return Err(VmError::from(VmErrorKind::BadArgumentCount {
            actual: args,
            expected: 1,
        }));
    }

    let value = stack.pop()?;
    let value = Value::Future(Shared::new(Future::new(join(value))));
    stack.push(value);
    Ok(())
}
