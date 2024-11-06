prelude!();

use std::sync::Arc;

#[test]
fn test_function() {
    let context = Arc::new(Context::with_default_modules().unwrap());

    // ptr to dynamic function.
    let function: Function = rune! {
        fn foo(a, b) { a + b }
        foo
    };

    assert_eq!(function.call::<i64>((1i64, 3i64)).unwrap(), 4i64);
    assert!(function.call::<i64>((1i64,)).is_err());

    // ptr to native function
    let function: Function = rune!(Vec::new);

    let value: Vec<Value> = function.call(()).unwrap();
    assert_eq!(value.len(), 0);

    // ptr to dynamic function.
    let function: Function = rune! {
        enum Custom { A(a) }
        Custom::A
    };

    assert!(function.call::<Value>(()).into_result().is_err());
    let value: Value = function.call((1i64,)).unwrap();
    assert!(rune::from_value::<DynamicTuple>(value).is_ok());

    // ptr to dynamic function.
    let function: Function = rune! {
        struct Custom(a);
        Custom
    };

    assert!(function.call::<Value>(()).into_result().is_err());
    let value: Value = function.call((1i64,)).unwrap();
    assert!(crate::from_value::<DynamicTuple>(value).is_ok());

    // non-capturing closure == free function
    let function: Function = rune! {
        |a, b| a + b
    };

    assert!(function.call::<Value>((1i64,)).into_result().is_err());
    let value: Value = function.call((1i64, 2i64)).unwrap();
    assert_eq!(value.as_signed().unwrap(), 3);

    // closure with captures
    let function: Function = run(
        &context,
        "pub fn main(a, b) { || a + b }",
        (1i64, 2i64),
        false,
    )
    .unwrap();

    assert!(function.call::<Value>((1i64,)).into_result().is_err());
    let value: Value = function.call(()).unwrap();
    assert_eq!(value.as_signed().unwrap(), 3);
}
