prelude!();

use std::sync::Arc;

#[test]
fn test_function() {
    let context = Arc::new(Context::with_default_modules().unwrap());

    // ptr to dynamic function.
    let function: Function = rune! {
        fn foo(a, b) { a + b }

        pub fn main() { foo }
    };

    assert_eq!(function.call::<_, i64>((1i64, 3i64)).unwrap(), 4i64);
    assert!(function.call::<_, i64>((1i64,)).is_err());

    // ptr to native function
    let function: Function = rune!(
        pub fn main() {
            Vec::new
        }
    );

    let value: Vec<Value> = function.call(()).unwrap();
    assert_eq!(value.len(), 0);

    // ptr to dynamic function.
    let function: Function = rune! {
        enum Custom { A(a) }
        pub fn main() { Custom::A }
    };

    assert!(function.call::<_, Value>(()).into_result().is_err());
    let value: Value = function.call((1i64,)).unwrap();
    assert!(matches!(value, Value::Variant(..)));

    // ptr to dynamic function.
    let function: Function = rune! {
        struct Custom(a);
        pub fn main() { Custom }
    };

    assert!(function.call::<_, Value>(()).into_result().is_err());
    let value: Value = function.call((1i64,)).unwrap();
    assert!(matches!(value, Value::TupleStruct(..)));

    // non-capturing closure == free function
    let function: Function = rune! {
        pub fn main() { |a, b| a + b }
    };

    assert!(function.call::<_, Value>((1i64,)).into_result().is_err());
    let value: Value = function.call((1i64, 2i64)).unwrap();
    assert!(matches!(value, Value::Integer(3)));

    // closure with captures
    let function: Function = run(
        &context,
        r#"pub fn main(a, b) { || a + b }"#,
        ["main"],
        (1i64, 2i64),
    )
    .unwrap();

    assert!(function.call::<_, Value>((1i64,)).into_result().is_err());
    let value: Value = function.call(()).unwrap();
    assert!(matches!(value, Value::Integer(3)));
}
