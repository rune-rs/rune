use rust_alloc::boxed::Box;

use super::Dynamic;

#[test]
fn dynamic_drop() {
    let header = Box::new(42u32);
    let v1 = crate::to_value([1u32, 2, 3, 4]).unwrap();
    let dynamic = Dynamic::new(header, [v1]).unwrap();
}

#[test]
fn dynamic_borrow_ref() {
    let header = Box::new(42u32);
    let v1 = crate::to_value([1u32, 2, 3, 4]).unwrap();
    let dynamic = Dynamic::new(header, [v1]).unwrap();

    let values = dynamic.borrow_ref().unwrap();
    let values2 = dynamic.borrow_ref().unwrap();

    assert!(dynamic.borrow_mut().is_err());
    drop(values);
    assert!(dynamic.borrow_mut().is_err());
    drop(values2);
    assert!(dynamic.borrow_mut().is_ok());
}

#[test]
fn dynamic_borrow_mut() {
    let header = Box::new(42u32);
    let v1 = crate::to_value([1u32, 2, 3, 4]).unwrap();
    let dynamic = Dynamic::new(header, [v1]).unwrap();

    let mut values = dynamic.borrow_mut().unwrap();

    assert!(dynamic.borrow_ref().is_err());
    drop(values);
    assert!(dynamic.borrow_ref().is_ok());
}
