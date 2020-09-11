use runestick::{Any, Shared};

#[global_allocator]
static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();

#[derive(Debug, PartialEq, Eq)]
struct Foo(isize);

#[checkers::test]
fn test_take() {
    let thing = Shared::new(Any::new(Foo(0)));
    let _ = thing.take().unwrap();
}

#[checkers::test]
fn test_clone_take() {
    let thing = Shared::new(Any::new(Foo(0)));
    let thing2 = thing.clone();
    assert_eq!(Foo(0), thing2.take_downcast::<Foo>().unwrap());
    assert!(thing.take().is_err());
}

#[checkers::test]
fn test_from_ref() {
    let value = 10u32;

    unsafe {
        let (shared, guard) = Shared::from_ref(&value);
        assert!(shared.downcast_borrow_mut::<u32>().is_err());
        assert_eq!(&10u32, &*shared.downcast_borrow_ref::<u32>().unwrap());

        drop(guard);

        assert!(shared.downcast_borrow_mut::<u32>().is_err());
        assert!(shared.downcast_borrow_ref::<u32>().is_err());
    }
}

#[checkers::test]
fn test_from_mut() {
    let mut value = 10u32;

    unsafe {
        let (shared, guard) = Shared::from_mut(&mut value);
        *shared.downcast_borrow_mut::<u32>().unwrap() = 20;

        assert_eq!(&20u32, &*shared.downcast_borrow_mut::<u32>().unwrap());
        assert_eq!(&20u32, &*shared.downcast_borrow_ref::<u32>().unwrap());

        drop(guard);

        assert!(shared.downcast_borrow_mut::<u32>().is_err());
        assert!(shared.downcast_borrow_ref::<u32>().is_err());
    }
}
