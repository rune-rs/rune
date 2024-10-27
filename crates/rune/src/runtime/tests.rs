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

use super::{Access, AnyObj, Bytes, Mut, Ref, Shared, TypeHash, Value, VmResult};

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
    let _ = thing.into_any_obj()?;
    Ok(())
}

#[test]
fn test_clone_take() -> Result<()> {
    let v = Value::from(AnyObj::new(Thing(0))?);
    let v2 = v.clone();
    let v3 = v.clone();
    assert_eq!(Thing(0), v2.into_any::<Thing>()?);
    assert!(v3.into_any::<Thing>().is_err());
    let any = v.into_any_obj()?;
    assert_eq!(any.type_hash(), Thing::HASH);
    Ok(())
}

#[test]
fn test_from_ref() -> Result<()> {
    let value = Thing(10u32);

    unsafe {
        let (value, guard) = Value::from_ref(&value)?;
        assert!(value.downcast_borrow_mut::<Thing>().is_err());
        assert_eq!(10u32, value.downcast_borrow_ref::<Thing>()?.0);

        drop(guard);

        assert!(value.downcast_borrow_mut::<Thing>().is_err());
        assert!(value.downcast_borrow_ref::<Thing>().is_err());
    }

    Ok(())
}

#[test]
fn test_from_mut() -> Result<()> {
    let mut value = Thing(10u32);

    unsafe {
        let (value, guard) = Value::from_mut(&mut value)?;
        value.downcast_borrow_mut::<Thing>()?.0 = 20;

        assert_eq!(20u32, value.downcast_borrow_mut::<Thing>()?.0);
        assert_eq!(20u32, value.downcast_borrow_ref::<Thing>()?.0);

        drop(guard);

        assert!(value.downcast_borrow_mut::<Thing>().is_err());
        assert!(value.downcast_borrow_ref::<Thing>().is_err());
    }

    Ok(())
}

#[test]
fn shared_take() -> crate::support::Result<()> {
    let a = Shared::new(Count(0))?;
    let b = a.clone();

    {
        let mut a = a.borrow_mut()?;
        // NB: this is prevented since we have a live reference.
        assert!(b.take().is_err());
        a.0 += 1;
    }

    let a = a.take()?;
    assert_eq!(a.0, 1);
    Ok(())
}

#[test]
fn shared_borrow_ref() -> crate::support::Result<()> {
    let a = Shared::new(Count(0))?;

    a.borrow_mut()?.0 += 1;

    {
        let a_ref = a.borrow_ref()?;
        assert_eq!(a_ref.0, 1);
        assert!(a.borrow_mut().is_err());
        assert!(a.borrow_ref().is_ok());
    }

    let mut a = a.borrow_mut()?;
    a.0 += 1;
    assert_eq!(a.0, 2);
    Ok(())
}

#[test]
fn shared_borrow_mut() -> crate::support::Result<()> {
    let a = Shared::new(Count(0))?;

    {
        let mut a_mut = a.borrow_mut()?;
        a_mut.0 += 1;
        assert_eq!(a_mut.0, 1);
        assert!(a.borrow_ref().is_err());
    }

    let a = a.borrow_ref()?;
    assert_eq!(a.0, 1);
    Ok(())
}

#[test]
fn shared_into_mut_raw() -> crate::support::Result<()> {
    let value = Shared::new(Count(42))?;

    let (mut ptr, guard) = Mut::into_raw(value.clone().into_mut()?);

    assert_eq!(value.snapshot().shared(), 0);
    assert!(value.snapshot().is_exclusive());

    // SAFETY: The guard is held.
    unsafe {
        assert_eq!(*ptr.as_ref(), Count(42));
        *ptr.as_mut() = Count(43);
    }

    drop(guard);

    assert_eq!(value.snapshot().shared(), 0);
    assert!(!value.snapshot().is_exclusive());

    assert_eq!(*value.borrow_ref()?, Count(43));
    Ok(())
}

#[test]
fn shared_into_ref() -> crate::support::Result<()> {
    let a = Shared::new(Count(0))?;
    let b = a.clone();

    b.borrow_mut()?.0 += 1;

    {
        // Consumes `a`.
        let a = a.into_ref()?;
        assert_eq!(a.0, 1);
        assert!(b.borrow_mut().is_err());
    }

    let mut b = b.borrow_mut()?;
    b.0 += 1;
    assert_eq!(b.0, 2);
    Ok(())
}

#[test]
fn shared_into_ref_map() -> crate::support::Result<()> {
    let value = Shared::<Vec<u32>>::new(try_vec![1, 2, 3, 4])?;

    let values = Ref::map(value.clone().into_ref()?, |value| &value[2..]);

    assert_eq!(value.snapshot().shared(), 1);
    assert!(!value.snapshot().is_exclusive());

    assert_eq!(values.len(), 2);
    assert_eq!(values.as_ref(), &[3, 4]);

    drop(values);

    assert_eq!(value.snapshot().shared(), 0);
    assert!(!value.snapshot().is_exclusive());

    assert_eq!(*value.borrow_ref()?, [1, 2, 3, 4]);
    Ok(())
}

#[test]
fn shared_into_ref_raw() -> crate::support::Result<()> {
    let value = Shared::new(Count(42))?;

    let (ptr, guard) = Ref::into_raw(value.clone().into_ref()?);

    assert_eq!(value.snapshot().shared(), 1);
    assert!(!value.snapshot().is_exclusive());

    // SAFETY: The guard is held.
    unsafe {
        assert_eq!(*ptr.as_ref(), Count(42));
    }

    drop(guard);

    assert_eq!(value.snapshot().shared(), 0);
    assert!(!value.snapshot().is_exclusive());

    assert_eq!(*value.borrow_ref()?, Count(42));
    Ok(())
}

#[test]
fn shared_into_mut() -> crate::support::Result<()> {
    let a = Shared::new(Count(0))?;
    let b = a.clone();

    {
        // Consumes `a`.
        let mut a = a.into_mut().unwrap();
        a.0 += 1;

        assert!(b.borrow_ref().is_err());
    }

    assert_eq!(b.borrow_ref().unwrap().0, 1);
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

#[test]
fn any_ref_from_own() {
    let v = Thing(1u32);

    let any = AnyObj::new(v).unwrap();
    let b = any.downcast_borrow_ref::<Thing>().unwrap();
    assert_eq!(b.0, 1u32);
    drop(b);

    let mut b = any.downcast_borrow_mut::<Thing>().unwrap();
    b.0 += 1;
    assert_eq!(b.0, 2u32);

    assert!(any.downcast_borrow_ref::<Thing>().is_err());
    drop(b);

    let b = any.downcast_borrow_ref::<Thing>().unwrap();
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
    let b = any.downcast_borrow_ref::<Boxed>().unwrap();
    assert_eq!(*b.0, 1u32);
    drop(b);

    let mut b = any.downcast_borrow_mut::<Boxed>().unwrap();
    *b.0 += 1;
    assert_eq!(*b.0, 2u32);

    assert!(any.downcast_borrow_ref::<Boxed>().is_err());
    drop(b);

    let b = any.downcast_borrow_ref::<Boxed>().unwrap();
    assert_eq!(*b.0, 2u32);
    drop(b);

    let v = any.downcast::<Boxed>().unwrap();
    assert_eq!(*v.0, 2u32);
}

#[test]
fn any_ref_from_ref() {
    let v = Thing(1u32);

    let any = unsafe { AnyObj::from_ref(&v).unwrap() };
    let b = any.downcast_borrow_ref::<Thing>().unwrap();
    assert_eq!(b.0, 1u32);
    drop(b);

    assert!(any.downcast::<Thing>().is_err());
}

#[test]
fn any_ref_downcast_borrow_ref() {
    let t = Thing(1u32);

    let any = unsafe { AnyObj::from_ref(&t).unwrap() };

    assert_eq!(
        Ok(&Thing(1u32)),
        any.downcast_borrow_ref::<Thing>().as_deref()
    );

    assert!(any.downcast::<Thing>().is_err());
}

#[test]
fn any_ref_from_mut() {
    let mut v = Thing(1u32);

    let any = unsafe { AnyObj::from_mut(&mut v).unwrap() };
    any.downcast_borrow_mut::<Thing>().unwrap().0 += 1;

    assert_eq!(v.0, 2);

    assert!(any.downcast::<Thing>().is_err());
}

#[test]
fn any_ref_downcast_borrow_mut() {
    let mut t = Thing(1u32);

    let any = unsafe { AnyObj::from_mut(&mut t).unwrap() };
    any.downcast_borrow_mut::<Thing>().unwrap().0 = 2;

    assert_eq!(
        Ok(&Thing(2u32)),
        any.downcast_borrow_ref::<Thing>().as_deref()
    );

    assert!(any.downcast::<Thing>().is_err());
}

#[test]
fn value_from_mut() {
    let mut v = Count(1);

    unsafe {
        let (any, guard) = Value::from_mut(&mut v).unwrap();

        if let Ok(mut v) = any.downcast_borrow_mut::<Count>() {
            v.0 += 1;
        }

        drop(guard);
        assert!(any.downcast_borrow_mut::<Count>().is_err());
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
        let shared = shared.into_bytes_ref().unwrap();
        let out = shared.try_clone().unwrap();
        out
    };
}
