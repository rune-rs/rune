use crate as rune;
use crate::runtime::{AnyObj, Shared};
use crate::Any;

#[derive(Any, Debug, PartialEq, Eq)]
struct Foo(isize);

#[test]
fn test_take() {
    let thing = Shared::new(AnyObj::new(Foo(0))).unwrap();
    let _ = thing.take().unwrap();
}

#[test]
fn test_clone_take() {
    let thing = Shared::new(AnyObj::new(Foo(0))).unwrap();
    let thing2 = thing.clone();
    assert_eq!(Foo(0), thing2.take_downcast::<Foo>().unwrap());
    assert!(thing.take().is_err());
}

#[test]
fn test_from_ref() {
    #[derive(Any)]
    struct Thing(u32);

    let value = Thing(10u32);

    unsafe {
        let (shared, guard) = Shared::from_ref(&value).unwrap();
        assert!(shared.downcast_borrow_mut::<Thing>().is_err());
        assert_eq!(10u32, shared.downcast_borrow_ref::<Thing>().unwrap().0);

        drop(guard);

        assert!(shared.downcast_borrow_mut::<Thing>().is_err());
        assert!(shared.downcast_borrow_ref::<Thing>().is_err());
    }
}

#[test]
fn test_from_mut() {
    #[derive(Any)]
    struct Thing(u32);

    let mut value = Thing(10u32);

    unsafe {
        let (shared, guard) = Shared::from_mut(&mut value).unwrap();
        shared.downcast_borrow_mut::<Thing>().unwrap().0 = 20;

        assert_eq!(20u32, shared.downcast_borrow_mut::<Thing>().unwrap().0);
        assert_eq!(20u32, shared.downcast_borrow_ref::<Thing>().unwrap().0);

        drop(guard);

        assert!(shared.downcast_borrow_mut::<Thing>().is_err());
        assert!(shared.downcast_borrow_ref::<Thing>().is_err());
    }
}
