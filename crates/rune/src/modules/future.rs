//! The `std::future` module.

use crate::alloc::Vec;
use crate::runtime::{Future, SelectFuture, Shared, Stack, Value, VmErrorKind, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::future` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["future"])?;
    module.ty::<Future>()?;

    module
        .raw_function("join", raw_join)
        .build()?
        .is_async(true)
        .args(1)
        .argument_types([None])?
        .docs([
            "Waits for a collection of futures to complete and joins their result.",
            "",
            "# Examples",
            "",
            "```rune",
            "let a = async { 1 };",
            "let b = async { 2 };",
            "let (a, b) = std::future::join((a, b)).await;",
            "assert_eq!(1, a);",
            "assert_eq!(2, b);",
            "```",
            "",
            "Using a vector:",
            "",
            "```rune",
            "let a = async { 1 };",
            "let b = async { 2 };",
            "let [a, b] = std::future::join([a, b]).await;",
            "assert_eq!(1, a);",
            "assert_eq!(2, b);",
            "```",
            "",
            "Joining an empty collection:",
            "",
            "```rune",
            "let () = std::future::join(()).await;",
            "let [] = std::future::join([]).await;",
            "```",
        ])?;

    Ok(module)
}

async fn try_join_impl<'a, I, F>(values: I, len: usize, factory: F) -> VmResult<Value>
where
    I: IntoIterator<Item = &'a Value>,
    F: FnOnce(Vec<Value>) -> VmResult<Value>,
{
    use futures_util::stream::StreamExt as _;

    let mut futures = futures_util::stream::FuturesUnordered::new();
    let mut results = vm_try!(Vec::try_with_capacity(len));

    for (index, value) in values.into_iter().enumerate() {
        let future = match value {
            Value::Future(future) => vm_try!(future.clone().into_mut()),
            value => {
                return VmResult::err([
                    VmErrorKind::expected::<Future>(vm_try!(value.type_info())),
                    VmErrorKind::bad_argument(index),
                ])
            }
        };

        futures.push(SelectFuture::new(index, future));
        vm_try!(results.try_push(Value::EmptyTuple));
    }

    while !futures.is_empty() {
        let (index, value) = vm_try!(futures.next().await.unwrap());
        *results.get_mut(index).unwrap() = value;
    }

    factory(results)
}

async fn join(value: Value) -> VmResult<Value> {
    match value {
        Value::EmptyTuple => VmResult::Ok(Value::EmptyTuple),
        Value::Tuple(tuple) => {
            let tuple = vm_try!(tuple.borrow_ref());
            VmResult::Ok(vm_try!(
                try_join_impl(tuple.iter(), tuple.len(), |vec| VmResult::Ok(vm_try!(
                    Value::tuple(vec)
                )))
                .await
            ))
        }
        Value::Vec(vec) => {
            let vec = vm_try!(vec.borrow_ref());
            VmResult::Ok(vm_try!(
                try_join_impl(vec.iter(), vec.len(), Value::vec).await
            ))
        }
        actual => VmResult::err([
            VmErrorKind::bad_argument(0),
            VmErrorKind::expected::<crate::runtime::Vec>(vm_try!(actual.type_info())),
        ]),
    }
}

/// The join implementation.
fn raw_join(stack: &mut Stack, args: usize) -> VmResult<()> {
    if args != 1 {
        return VmResult::err(VmErrorKind::BadArgumentCount {
            actual: args,
            expected: 1,
        });
    }

    let value = vm_try!(stack.pop());
    let value = Value::Future(vm_try!(Shared::new(vm_try!(Future::new(join(value))))));
    vm_try!(stack.push(value));
    VmResult::Ok(())
}
