//! Asynchronous computations.

use crate as rune;
use crate::alloc::Vec;
use crate::runtime::{self, Future, Inline, Repr, SelectFuture, Value, VmError, VmErrorKind};
use crate::{ContextError, Module, TypeHash};

/// Asynchronous computations.
#[rune::module(::std::future)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module__meta)?;
    module.ty::<Future>()?;
    module.function_meta(join__meta)?;
    Ok(module)
}

async fn try_join_impl<'a, I, F>(values: I, len: usize, factory: F) -> Result<Value, VmError>
where
    I: IntoIterator<Item = &'a Value>,
    F: FnOnce(Vec<Value>) -> Result<Value, VmError>,
{
    use futures_util::stream::StreamExt as _;

    let mut futures = futures_util::stream::FuturesUnordered::new();
    let mut results = Vec::try_with_capacity(len)?;

    for (index, value) in values.into_iter().enumerate() {
        match value.as_ref() {
            Repr::Inline(value) => {
                return Err(VmError::from([
                    VmErrorKind::expected::<Future>(value.type_info()),
                    VmErrorKind::bad_argument(index),
                ]));
            }
            Repr::Dynamic(value) => {
                return Err(VmError::from([
                    VmErrorKind::expected::<Future>(value.type_info()),
                    VmErrorKind::bad_argument(index),
                ]));
            }
            Repr::Any(value) => match value.type_hash() {
                Future::HASH => {
                    let future = Value::from(value.clone()).into_future()?;
                    futures.push(SelectFuture::new(index, future));
                    results.try_push(Value::empty())?;
                }
                _ => {
                    return Err(VmError::from([
                        VmErrorKind::expected::<Future>(value.type_info()),
                        VmErrorKind::bad_argument(index),
                    ]));
                }
            },
        }
    }

    while !futures.is_empty() {
        let (index, value) = futures.next().await.unwrap()?;
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
async fn join(value: Value) -> Result<Value, VmError> {
    match value.as_ref() {
        Repr::Inline(value) => match value {
            Inline::Unit => Ok(Value::unit()),
            value => Err(VmError::from([
                VmErrorKind::bad_argument(0),
                VmErrorKind::expected::<runtime::Vec>(value.type_info()),
            ])),
        },
        Repr::Dynamic(value) => Err(VmError::from([
            VmErrorKind::bad_argument(0),
            VmErrorKind::expected::<runtime::Vec>(value.type_info()),
        ])),
        Repr::Any(value) => match value.type_hash() {
            runtime::Vec::HASH => {
                let vec = value.borrow_ref::<runtime::Vec>()?;
                let result = try_join_impl(vec.iter(), vec.len(), |vec| {
                    Value::vec(vec).map_err(VmError::from)
                })
                .await;
                Ok(result?)
            }
            runtime::OwnedTuple::HASH => {
                let tuple = value.borrow_ref::<runtime::OwnedTuple>()?;

                let result = try_join_impl(tuple.iter(), tuple.len(), |vec| {
                    Value::tuple(vec).map_err(VmError::from)
                })
                .await;

                Ok(result?)
            }
            _ => Err(VmError::from([
                VmErrorKind::bad_argument(0),
                VmErrorKind::expected::<runtime::Vec>(value.type_info()),
            ])),
        },
    }
}
