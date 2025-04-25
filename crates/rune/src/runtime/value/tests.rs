use rust_alloc::boxed::Box;

use super::AnySequence;

#[derive(Debug, PartialEq, Eq)]
struct Count(isize);

#[test]
fn dynamic_drop() {
    let header = Box::new(42u32);
    let v1 = crate::to_value([1u32, 2, 3, 4]).unwrap();
    let _dynamic = AnySequence::new(header, [v1]).unwrap();
}

#[test]
fn dynamic_borrow_ref() {
    let header = Box::new(42u32);
    let v1 = crate::to_value([1u32, 2, 3, 4]).unwrap();
    let dynamic = AnySequence::new(header, [v1]).unwrap();

    let values = dynamic.borrow_ref().unwrap();
    let values2 = dynamic.borrow_ref().unwrap();

    assert!(dynamic.borrow_mut().is_err());
    drop(values);
    assert!(dynamic.borrow_mut().is_err());
    drop(values2);
    assert!(dynamic.borrow_mut().is_ok());
}

#[test]
fn dynamic_borrow_ref_err() -> crate::support::Result<()> {
    let a = AnySequence::new((), [Count(0)])?;

    a.borrow_mut()?[0].0 += 1;

    {
        let a_ref = a.borrow_ref()?;
        assert_eq!(a_ref[0].0, 1);
        assert!(a.borrow_mut().is_err());
        assert!(a.borrow_ref().is_ok());
    }

    let mut a = a.borrow_mut()?;
    a[0].0 += 1;
    assert_eq!(a[0].0, 2);
    Ok(())
}

#[test]
fn dynamic_borrow_mut() {
    let header = Box::new(42u32);
    let v1 = crate::to_value([1u32, 2, 3, 4]).unwrap();
    let dynamic = AnySequence::new(header, [v1]).unwrap();

    let values = dynamic.borrow_mut().unwrap();

    assert!(dynamic.borrow_ref().is_err());
    drop(values);
    assert!(dynamic.borrow_ref().is_ok());
}

#[test]
fn dynamic_borrow_mut_err() -> crate::support::Result<()> {
    let a = AnySequence::new((), [Count(0)])?;

    {
        let mut a_mut = a.borrow_mut()?;
        a_mut[0].0 += 1;
        assert_eq!(a_mut[0].0, 1);
        assert!(a.borrow_ref().is_err());
    }

    let a = a.borrow_ref()?;
    assert_eq!(a[0].0, 1);
    Ok(())
}

#[test]
fn dynamic_take() -> crate::support::Result<()> {
    let a = AnySequence::new((), [Count(0)])?;
    let b = a.clone();

    {
        let mut a = a.borrow_mut()?;
        // NB: this is prevented since we have a live reference.
        assert!(b.take().is_err());
        a[0].0 += 1;
    }

    let a = a.take()?;
    assert_eq!(a.borrow_ref()?[0].0, 1);
    Ok(())
}

#[test]
fn dynamic_is_readable() -> crate::support::Result<()> {
    let dynamic = AnySequence::new((), [1u32])?;
    assert!(dynamic.is_readable());

    {
        let _guard = dynamic.borrow_ref()?;
        assert!(dynamic.is_readable()); // Note: still readable.
    }

    {
        let _guard = dynamic.borrow_mut()?;
        assert!(!dynamic.is_readable());
    }

    assert!(dynamic.is_readable());
    Ok(())
}

#[test]
fn dynamic_is_writable_take() -> crate::support::Result<()> {
    let shared = AnySequence::new((), [1u32])?;
    let shared2 = shared.clone();
    assert!(shared.is_readable());
    shared.take()?;
    assert!(!shared2.is_readable());
    assert!(shared2.take().is_err());
    Ok(())
}

#[test]
fn dynamic_is_writable() -> crate::support::Result<()> {
    let shared = AnySequence::new((), [1u32])?;
    assert!(shared.is_writable());

    {
        let _guard = shared.borrow_ref()?;
        assert!(!shared.is_writable());
    }

    assert!(shared.is_writable());
    Ok(())
}
