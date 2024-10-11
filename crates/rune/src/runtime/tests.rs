use core::future::Future as _;
use core::pin::pin;
use core::task::{Context, Poll};

use std::sync::Arc;
use std::task::Wake;

use crate as rune;

use crate::runtime::{AnyObj, Shared, Value, VmResult};
use crate::Any;

use crate::support::Result;

struct NoopWaker;

impl Wake for NoopWaker {
    fn wake(self: Arc<Self>) {
        // nothing
    }
}

#[derive(Any, Debug, PartialEq, Eq)]
struct Foo(isize);

#[test]
fn test_take() -> Result<()> {
    let thing = Value::try_from(AnyObj::new(Foo(0))?)?;
    let _ = thing.into_any_obj()?;
    Ok(())
}

#[test]
fn test_clone_take() -> Result<()> {
    let thing = Value::try_from(AnyObj::new(Foo(0))?)?;
    let thing2 = thing.clone();
    assert_eq!(Foo(0), thing2.into_any::<Foo>()?);
    assert!(thing.into_any_obj().is_err());
    Ok(())
}

#[test]
fn test_from_ref() -> Result<()> {
    #[derive(Any)]
    struct Thing(u32);

    let value = Thing(10u32);

    unsafe {
        let (value, guard) = Value::from_ref(&value)?;
        assert!(value.clone().into_any_mut::<Thing>().is_err());
        assert_eq!(10u32, value.clone().into_any_ref::<Thing>()?.0);

        drop(guard);

        assert!(value.clone().into_any_mut::<Thing>().is_err());
        assert!(value.clone().into_any_ref::<Thing>().is_err());
    }

    Ok(())
}

#[test]
fn test_from_mut() -> Result<()> {
    #[derive(Any)]
    struct Thing(u32);

    let mut value = Thing(10u32);

    unsafe {
        let (value, guard) = Value::from_mut(&mut value)?;
        value.clone().into_any_mut::<Thing>()?.0 = 20;

        assert_eq!(20u32, value.clone().into_any_mut::<Thing>()?.0);
        assert_eq!(20u32, value.clone().into_any_ref::<Thing>()?.0);

        drop(guard);

        assert!(value.clone().into_any_mut::<Thing>().is_err());
        assert!(value.clone().into_any_ref::<Thing>().is_err());
    }

    Ok(())
}

#[test]
fn shared_take() -> crate::support::Result<()> {
    #[derive(Debug)]
    struct Foo {
        counter: isize,
    }

    let a = Shared::new(Foo { counter: 0 })?;
    let b = a.clone();

    {
        let mut a = a.borrow_mut()?;
        // NB: this is prevented since we have a live reference.
        assert!(b.take().is_err());
        a.counter += 1;
    }

    let a = a.take()?;
    assert_eq!(a.counter, 1);
    Ok(())
}

#[test]
fn shared_borrow_ref() -> crate::support::Result<()> {
    #[derive(Debug)]
    struct Foo {
        counter: isize,
    }

    let a = Shared::new(Foo { counter: 0 })?;

    a.borrow_mut()?.counter += 1;

    {
        let a_ref = a.borrow_ref()?;
        assert_eq!(a_ref.counter, 1);
        assert!(a.borrow_mut().is_err());
        assert!(a.borrow_ref().is_ok());
    }

    let mut a = a.borrow_mut()?;
    a.counter += 1;
    assert_eq!(a.counter, 2);
    Ok(())
}

#[test]
fn shared_borrow_mut() -> crate::support::Result<()> {
    #[derive(Debug)]
    struct Foo {
        counter: isize,
    }

    let a = Shared::new(Foo { counter: 0 })?;

    {
        let mut a_mut = a.borrow_mut()?;
        a_mut.counter += 1;
        assert_eq!(a_mut.counter, 1);
        assert!(a.borrow_ref().is_err());
    }

    let a = a.borrow_ref()?;
    assert_eq!(a.counter, 1);
    Ok(())
}

#[test]
fn shared_into_ref() -> crate::support::Result<()> {
    #[derive(Debug)]
    struct Foo {
        counter: isize,
    }

    let a = Shared::new(Foo { counter: 0 })?;
    let b = a.clone();

    b.borrow_mut()?.counter += 1;

    {
        // Consumes `a`.
        let a = a.into_ref()?;
        assert_eq!(a.counter, 1);
        assert!(b.borrow_mut().is_err());
    }

    let mut b = b.borrow_mut()?;
    b.counter += 1;
    assert_eq!(b.counter, 2);
    Ok(())
}

#[test]
fn shared_into_mut() -> crate::support::Result<()> {
    #[derive(Debug)]
    struct Foo {
        counter: isize,
    }

    let a = Shared::new(Foo { counter: 0 })?;
    let b = a.clone();

    {
        // Consumes `a`.
        let mut a = a.into_mut().unwrap();
        a.counter += 1;

        assert!(b.borrow_ref().is_err());
    }

    assert_eq!(b.borrow_ref().unwrap().counter, 1);
    Ok(())
}

#[test]
fn shared_is_readable() -> crate::support::Result<()> {
    let shared = Shared::new(1u32)?;
    assert!(shared.is_readable());

    {
        let _guard = shared.borrow_ref()?;
        assert!(shared.is_readable()); // Note: still readable.
    }

    {
        let _guard = shared.borrow_mut()?;
        assert!(!shared.is_readable());
    }

    assert!(shared.is_readable());
    Ok(())
}

#[test]
fn shared_is_writable_take() -> crate::support::Result<()> {
    let shared = Shared::new(1u32)?;
    let shared2 = shared.clone();
    assert!(shared.is_readable());
    shared.take()?;
    assert!(!shared2.is_readable());
    assert!(shared2.take().is_err());
    Ok(())
}

#[test]
fn shared_is_writable() -> crate::support::Result<()> {
    let shared = Shared::new(1u32)?;
    assert!(shared.is_writable());

    {
        let _guard = shared.borrow_ref()?;
        assert!(!shared.is_writable());
    }

    assert!(shared.is_writable());
    Ok(())
}

#[test]
fn ensure_future_dropped_poll() -> crate::support::Result<()> {
    use crate::runtime::Future;

    let mut future = pin!(Future::new(async { VmResult::Ok(10) })?);

    let waker = Arc::new(NoopWaker).into();
    let mut cx = Context::from_waker(&waker);

    assert!(!future.is_completed());

    // NB: By polling the future to completion we are causing it to be dropped when polling is completed.
    let Poll::Ready(ok) = future.as_mut().poll(&mut cx) else {
        panic!("expected ready");
    };

    assert_eq!(ok.unwrap().as_integer().unwrap(), 10);
    assert!(future.is_completed());
    Ok(())
}

#[test]
fn ensure_future_dropped_explicitly() -> crate::support::Result<()> {
    use crate::runtime::Future;

    let mut future = pin!(Future::new(async { VmResult::Ok(10) })?);
    // NB: We cause the future to be dropped explicitly through it's Drop destructor here by replacing it.
    future.set(Future::new(async { VmResult::Ok(0) })?);

    let waker = Arc::new(NoopWaker).into();
    let mut cx = Context::from_waker(&waker);

    assert!(!future.is_completed());

    let Poll::Ready(ok) = future.as_mut().poll(&mut cx) else {
        panic!("expected ready");
    };

    assert_eq!(ok.unwrap().as_integer().unwrap(), 0);
    assert!(future.is_completed());
    Ok(())
}
