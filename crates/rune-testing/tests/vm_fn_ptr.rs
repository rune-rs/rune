use rune_testing::*;

#[test]
fn test_fn_ptr() {
    // ptr to dynamic function.
    let fn_ptr = rune! {
        FnPtr => r#"
        fn foo(a, b) {
            a + b
        }

        fn main() {
            foo
        }
        "#
    };

    assert_eq!(fn_ptr.call::<_, i64>((1i64, 3i64)).unwrap(), 4i64);
    assert!(fn_ptr.call::<_, i64>((1i64,)).is_err());

    // ptr to native function
    let fn_ptr = rune! {
        FnPtr => r#"fn main() { Vec::new }"#
    };

    let value: Vec<Value> = fn_ptr.call(()).unwrap();
    assert_eq!(value.len(), 0);

    // ptr to dynamic function.
    let fn_ptr = rune! {
        FnPtr => r#"
        enum Custom { A(a) }
        fn main() { Custom::A }
        "#
    };

    assert!(fn_ptr.call::<_, Value>(()).is_err());
    let value: Value = fn_ptr.call((1i64,)).unwrap();
    assert!(matches!(value, Value::VariantTuple(..)));

    // ptr to dynamic function.
    let fn_ptr = rune! {
        FnPtr => r#"
        struct Custom(a)
        fn main() { Custom }
        "#
    };

    assert!(fn_ptr.call::<_, Value>(()).is_err());
    let value: Value = fn_ptr.call((1i64,)).unwrap();
    assert!(matches!(value, Value::TypedTuple(..)));
}
