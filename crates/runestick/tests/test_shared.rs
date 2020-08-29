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
