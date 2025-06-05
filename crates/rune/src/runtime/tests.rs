use core::future::Future as _;
use core::pin::pin;
use core::task::{Context, Poll};

use std::boxed::Box;
use std::string::ToString;
use std::sync::Arc;
use std::task::Wake;

use crate as rune;

use crate::alloc::prelude::*;
use crate::support::Result;
use crate::Any;

use super::{Access, Address, AnyObj, Bytes, FunctionHandler, Output, Tuple, TypeHash, Value};

#[derive(Debug, PartialEq, Eq, Any)]
struct Thing(u32);

#[derive(Debug, PartialEq, Eq, Any)]
struct Boxed(Box<u32>);

#[derive(Debug, PartialEq, Eq, Any)]
struct Count(isize);

struct NoopWaker;

impl Wake for NoopWaker {
    fn wake(self: Arc<Self>) {
        // nothing
    }
}

#[test]
fn test_take() -> Result<()> {
    let thing = Value::from(AnyObj::new(Thing(0))?);
    let _ = thing.into_any_obj()?.take()?;
    Ok(())
}

#[test]
fn test_take_shared() -> Result<()> {
    let thing = Value::from(AnyObj::new(Thing(0))?);
    let shared = thing.into_shared::<Thing>()?;
    let inner = shared.take()?;
    assert_eq!(inner, Thing(0));
    Ok(())
}

#[test]
fn test_clone_take() -> Result<()> {
    let v = Value::from(AnyObj::new(Thing(0))?);
    let v2 = v.clone();
    let v3 = v.clone();
    assert_eq!(Thing(0), v2.downcast::<Thing>()?);
    assert!(v3.downcast::<Thing>().is_err());
    let any = v.into_any_obj()?;
    assert_eq!(any.type_hash(), Thing::HASH);
    Ok(())
}

#[test]
fn test_clone_take_shared() -> Result<()> {
    let v = Value::from(AnyObj::new(Thing(0))?);
    let v2 = v.clone();
    let v3 = v.clone().into_shared::<Thing>()?;
    assert_eq!(Thing(0), v2.downcast::<Thing>()?);
    assert!(v3.take().is_err());
    let any = v.into_any_obj()?;
    assert_eq!(any.type_hash(), Thing::HASH);
    Ok(())
}

#[test]
fn test_from_ref() -> Result<()> {
    let value = Thing(10u32);

    unsafe {
        let (value, guard) = Value::from_ref(&value)?;
        assert!(value.borrow_mut::<Thing>().is_err());
        assert_eq!(10u32, value.borrow_ref::<Thing>()?.0);

        drop(guard);

        assert!(value.borrow_mut::<Thing>().is_err());
        assert!(value.borrow_ref::<Thing>().is_err());
    }

    Ok(())
}

#[test]
fn test_from_mut() -> Result<()> {
    let mut value = Thing(10u32);

    unsafe {
        let (value, guard) = Value::from_mut(&mut value)?;
        value.borrow_mut::<Thing>()?.0 = 20;

        assert_eq!(20u32, value.borrow_mut::<Thing>()?.0);
        assert_eq!(20u32, value.borrow_ref::<Thing>()?.0);

        drop(guard);

        assert!(value.borrow_mut::<Thing>().is_err());
        assert!(value.borrow_ref::<Thing>().is_err());
    }

    Ok(())
}

#[test]
fn ensure_future_dropped_poll() -> crate::support::Result<()> {
    use crate::runtime::Future;

    let mut future = pin!(Future::new(async { Ok(10) })?);

    let waker = Arc::new(NoopWaker).into();
    let mut cx = Context::from_waker(&waker);

    assert!(!future.is_completed());

    // NB: By polling the future to completion we are causing it to be dropped when polling is completed.
    let Poll::Ready(ok) = future.as_mut().poll(&mut cx) else {
        panic!("expected ready");
    };

    assert_eq!(ok.unwrap().as_signed().unwrap(), 10);
    assert!(future.is_completed());
    Ok(())
}

#[test]
fn ensure_future_dropped_explicitly() -> crate::support::Result<()> {
    use crate::runtime::Future;

    let mut future = pin!(Future::new(async { Ok(10) })?);
    // NB: We cause the future to be dropped explicitly through it's Drop destructor here by replacing it.
    future.set(Future::new(async { Ok(0) })?);

    let waker = Arc::new(NoopWaker).into();
    let mut cx = Context::from_waker(&waker);

    assert!(!future.is_completed());

    let Poll::Ready(ok) = future.as_mut().poll(&mut cx) else {
        panic!("expected ready");
    };

    assert_eq!(ok.unwrap().as_signed().unwrap(), 0);
    assert!(future.is_completed());
    Ok(())
}

#[test]
fn any_ref_from_own() {
    let v = Thing(1u32);

    let any = AnyObj::new(v).unwrap();
    let b = any.borrow_ref::<Thing>().unwrap();
    assert_eq!(b.0, 1u32);
    drop(b);

    let mut b = any.borrow_mut::<Thing>().unwrap();
    b.0 += 1;
    assert_eq!(b.0, 2u32);

    assert!(any.borrow_ref::<Thing>().is_err());
    drop(b);

    let b = any.borrow_ref::<Thing>().unwrap();
    assert_eq!(b.0, 2u32);
    drop(b);

    let v = any.downcast::<Thing>().unwrap();
    assert_eq!(v.0, 2u32);
}

/// Test an any-ref which requires the inner value to be dropped.
#[test]
fn any_ref_from_own_boxed() {
    let v = Boxed(Box::new(1u32));

    let any = AnyObj::new(v).unwrap();
    let b = any.borrow_ref::<Boxed>().unwrap();
    assert_eq!(*b.0, 1u32);
    drop(b);

    let mut b = any.borrow_mut::<Boxed>().unwrap();
    *b.0 += 1;
    assert_eq!(*b.0, 2u32);

    assert!(any.borrow_ref::<Boxed>().is_err());
    drop(b);

    let b = any.borrow_ref::<Boxed>().unwrap();
    assert_eq!(*b.0, 2u32);
    drop(b);

    let v = any.downcast::<Boxed>().unwrap();
    assert_eq!(*v.0, 2u32);
}

#[test]
fn any_ref_from_ref() {
    let v = Thing(1u32);

    let any = unsafe { AnyObj::from_ref(&v).unwrap() };
    let b = any.borrow_ref::<Thing>().unwrap();
    assert_eq!(b.0, 1u32);
    drop(b);

    assert!(any.downcast::<Thing>().is_err());
}

#[test]
fn any_ref_downcast_borrow_ref() {
    let t = Thing(1u32);

    let any = unsafe { AnyObj::from_ref(&t).unwrap() };

    assert_eq!(Ok(&Thing(1u32)), any.borrow_ref::<Thing>().as_deref());

    assert!(any.downcast::<Thing>().is_err());
}

#[test]
fn any_ref_from_mut() {
    let mut v = Thing(1u32);

    let any = unsafe { AnyObj::from_mut(&mut v).unwrap() };
    any.borrow_mut::<Thing>().unwrap().0 += 1;

    assert_eq!(v.0, 2);

    assert!(any.downcast::<Thing>().is_err());
}

#[test]
fn any_ref_downcast_borrow_mut() {
    let mut t = Thing(1u32);

    let any = unsafe { AnyObj::from_mut(&mut t).unwrap() };
    any.borrow_mut::<Thing>().unwrap().0 = 2;

    assert_eq!(Ok(&Thing(2u32)), any.borrow_ref::<Thing>().as_deref());

    assert!(any.downcast::<Thing>().is_err());
}

#[test]
fn value_from_mut() {
    let mut v = Count(1);

    unsafe {
        let (any, guard) = Value::from_mut(&mut v).unwrap();

        if let Ok(mut v) = any.borrow_mut::<Count>() {
            v.0 += 1;
        }

        drop(guard);
        assert!(any.borrow_mut::<Count>().is_err());
        drop(any);
    }

    assert_eq!(v.0, 2);
}

#[test]
fn access_shared() {
    let access = Access::new();

    assert!(access.is_shared());
    assert!(access.is_exclusive());
    assert!(!access.is_taken());
    assert_eq!(access.snapshot().to_string(), "--000000");

    let g1 = access.shared().unwrap();
    let g2 = access.shared().unwrap();

    assert!(access.exclusive().is_err());
    assert!(access.try_take().is_err());

    assert!(access.is_shared());
    assert!(!access.is_exclusive());
    assert!(!access.is_taken());
    assert_eq!(access.snapshot().to_string(), "--000002");

    drop(g1);

    assert!(access.exclusive().is_err());
    assert!(access.try_take().is_err());

    assert!(access.is_shared());
    assert!(!access.is_exclusive());
    assert!(!access.is_taken());
    assert_eq!(access.snapshot().to_string(), "--000001");

    drop(g2);

    assert!(access.is_shared());
    assert!(access.is_exclusive());
    assert!(!access.is_taken());
    assert_eq!(access.snapshot().to_string(), "--000000");
}

#[test]
fn access_exclusive() {
    let access = Access::new();

    let guard = access.exclusive().unwrap();
    assert!(access.exclusive().is_err());
    assert!(access.try_take().is_err());

    assert!(!access.is_shared());
    assert!(!access.is_exclusive());
    assert!(!access.is_taken());
    assert_eq!(access.snapshot().to_string(), "-X000000");

    drop(guard);

    assert!(access.is_shared());
    assert!(access.is_exclusive());
    assert!(!access.is_taken());
    assert_eq!(access.snapshot().to_string(), "--000000");

    let guard = access.exclusive().unwrap();
    assert!(!access.is_shared());
    assert!(!access.is_exclusive());
    assert!(!access.is_taken());
    assert_eq!(access.snapshot().to_string(), "-X000000");

    drop(guard);
}

#[test]
fn access_try_take() {
    let access = Access::new();

    assert_eq!(access.snapshot().to_string(), "--000000");

    let guard = access.exclusive().unwrap();

    assert_eq!(access.snapshot().to_string(), "-X000000");

    assert!(access.try_take().is_err());
    drop(guard);

    assert_eq!(access.snapshot().to_string(), "--000000");

    assert!(access.is_shared());
    assert!(access.is_exclusive());
    assert!(!access.is_taken());

    access.try_take().unwrap();

    assert!(!access.is_shared());
    assert!(!access.is_exclusive());
    assert!(access.is_taken());

    assert_eq!(access.snapshot().to_string(), "M-000000");
}

#[test]
#[allow(clippy::let_and_return)]
fn test_clone_issue() {
    let shared = Value::try_from(Bytes::new()).unwrap();

    let _ = {
        let shared = shared.into_ref::<Bytes>().unwrap();
        let out = shared.try_clone().unwrap();
        out
    };
}

#[test]
fn test_drop_boxed_tuple() {
    let boxed =
        crate::alloc::Box::<[Value]>::try_from([Value::from(1u32), Value::from(2u64)]).unwrap();
    let boxed = Tuple::from_boxed(boxed);
    drop(boxed);
}

#[test]
fn test_function_handler() {
    use std::thread;

    let handler = FunctionHandler::new(|m, _addr, _count, _out| {
        *m.at_mut(Address::ZERO).unwrap() = Value::from(42u32);
        Ok(())
    })
    .unwrap();

    let handler2 = handler.clone();

    let t = thread::spawn(move || {
        let mut memory = [Value::empty()];
        handler
            .call(&mut memory, Address::ZERO, 0, Output::discard())
            .unwrap();
        let [value] = memory;
        value.as_integer::<u32>().unwrap()
    });

    let mut memory = [Value::empty()];
    handler2
        .call(&mut memory, Address::ZERO, 0, Output::discard())
        .unwrap();
    let [value] = memory;
    assert_eq!(value.as_integer::<u32>().unwrap(), 42);

    assert_eq!(t.join().unwrap(), 42);

    drop(handler2);
}
