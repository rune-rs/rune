//! Asynchronous computations.

use crate as rune;
use crate::alloc::Vec;
use crate::runtime::{self, Future, Inline, Repr, SelectFuture, Value, VmErrorKind, VmResult};
use crate::{ContextError, Module, TypeHash};

/// Asynchronous computations.
#[rune::module(::std::future)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.ty::<Future>()?;
    module.function_meta(join__meta)?;
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
        match value.as_ref() {
            Repr::Inline(value) => {
                return VmResult::err([
                    VmErrorKind::expected::<Future>(value.type_info()),
                    VmErrorKind::bad_argument(index),
                ]);
            }
            Repr::Dynamic(value) => {
                return VmResult::err([
                    VmErrorKind::expected::<Future>(value.type_info()),
                    VmErrorKind::bad_argument(index),
                ]);
            }
            Repr::Any(value) => match value.type_hash() {
                Future::HASH => {
                    let future = vm_try!(Value::from(value.clone()).into_future());
                    futures.push(SelectFuture::new(index, future));
                    vm_try!(results.try_push(Value::empty()));
                }
                _ => {
                    return VmResult::err([
                        VmErrorKind::expected::<Future>(value.type_info()),
                        VmErrorKind::bad_argument(index),
                    ]);
                }
            },
        }
    }

    while !futures.is_empty() {
        let (index, value) = vm_try!(futures.next().await.unwrap());
        *results.get_mut(index).unwrap() = value;
    }

    factory(results)
}

/// Waits for a collection of futures to complete and joins their result.
///
/// # Examples
///
/// ```rune
/// use std::future;
///
/// let a = async { 1 };
/// let b = async { 2 };
/// let (a, b) = future::join((a, b)).await;
/// assert_eq!(1, a);
/// assert_eq!(2, b);
/// ```
///
/// Using a vector:
///
/// ```rune
/// use std::future;
///
/// let a = async { 1 };
/// let b = async { 2 };
/// let [a, b] = future::join([a, b]).await;
/// assert_eq!(1, a);
/// assert_eq!(2, b);
/// ```
///
/// Joining an empty collection:
///
/// ```rune
/// use std::future;
///
/// let () = future::join(()).await;
/// let [] = future::join([]).await;
/// ```
#[rune::function(keep)]
async fn join(value: Value) -> VmResult<Value> {
    match value.as_ref() {
        Repr::Inline(value) => match value {
            Inline::Unit => VmResult::Ok(Value::unit()),
            value => VmResult::err([
                VmErrorKind::bad_argument(0),
                VmErrorKind::expected::<runtime::Vec>(value.type_info()),
            ]),
        },
        Repr::Dynamic(value) => VmResult::err([
            VmErrorKind::bad_argument(0),
            VmErrorKind::expected::<runtime::Vec>(value.type_info()),
        ]),
        Repr::Any(value) => match value.type_hash() {
            runtime::Vec::HASH => {
                let vec = vm_try!(value.borrow_ref::<runtime::Vec>());
                let result = try_join_impl(vec.iter(), vec.len(), |vec| {
                    VmResult::Ok(vm_try!(Value::vec(vec)))
                })
                .await;
                VmResult::Ok(vm_try!(result))
            }
            runtime::OwnedTuple::HASH => {
                let tuple = vm_try!(value.borrow_ref::<runtime::OwnedTuple>());

                let result = try_join_impl(tuple.iter(), tuple.len(), |vec| {
                    VmResult::Ok(vm_try!(Value::tuple(vec)))
                })
                .await;

                VmResult::Ok(vm_try!(result))
            }
            _ => VmResult::err([
                VmErrorKind::bad_argument(0),
                VmErrorKind::expected::<runtime::Vec>(value.type_info()),
            ]),
        },
    }
}
