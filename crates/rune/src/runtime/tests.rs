use crate as rune;
use crate::runtime::{AnyObj, Shared};
use crate::Any;

use crate::support::Result;

#[derive(Any, Debug, PartialEq, Eq)]
struct Foo(isize);

#[test]
fn test_take() -> Result<()> {
    let thing = Shared::new(AnyObj::new(Foo(0))?)?;
    let _ = thing.take().unwrap();
    Ok(())
}

#[test]
fn test_clone_take() -> Result<()> {
    let thing = Shared::new(AnyObj::new(Foo(0))?)?;
    let thing2 = thing.clone();
    assert_eq!(Foo(0), thing2.take_downcast::<Foo>()?);
    assert!(thing.take().is_err());
    Ok(())
}

#[test]
fn test_from_ref() -> Result<()> {
    #[derive(Any)]
    struct Thing(u32);

    let value = Thing(10u32);

    unsafe {
        let (shared, guard) = Shared::from_ref(&value)?;
        assert!(shared.downcast_borrow_mut::<Thing>().is_err());
        assert_eq!(10u32, shared.downcast_borrow_ref::<Thing>()?.0);

        drop(guard);

        assert!(shared.downcast_borrow_mut::<Thing>().is_err());
        assert!(shared.downcast_borrow_ref::<Thing>().is_err());
    }

    Ok(())
}

#[test]
fn test_from_mut() -> Result<()> {
    #[derive(Any)]
    struct Thing(u32);

    let mut value = Thing(10u32);

    unsafe {
        let (shared, guard) = Shared::from_mut(&mut value)?;
        shared.downcast_borrow_mut::<Thing>()?.0 = 20;

        assert_eq!(20u32, shared.downcast_borrow_mut::<Thing>()?.0);
        assert_eq!(20u32, shared.downcast_borrow_ref::<Thing>()?.0);

        drop(guard);

        assert!(shared.downcast_borrow_mut::<Thing>().is_err());
        assert!(shared.downcast_borrow_ref::<Thing>().is_err());
    }

    Ok(())
}
