use rune_testing::*;
use runestick::FromValue;

#[test]
fn test_from_value_object_like() {
    #[derive(FromValue)]
    struct Proxy {
        field: u32,
    }

    let value = rune! {
        Proxy => r#"
        struct Ignored;
        struct Value { field, ignored }
        fn main() { Value { field: 42, ignored: Ignored } }
        "#
    };

    assert_eq!(value.field, 42);

    let value = rune! {
        Proxy => r#"
        struct Ignored;
        fn main() { #{ field: 42, ignored: Ignored } }
        "#
    };

    assert_eq!(value.field, 42);
}

#[test]
fn test_from_value_tuple_like() {
    #[derive(FromValue)]
    struct Proxy(u32);

    let value = rune! {
        Proxy => r#"
        struct Value(field);
        fn main() { Value(42) }
        "#
    };

    assert_eq!(value.0, 42);

    let value = rune! {
        Proxy => r#"
        fn main() { (42,) }
        "#
    };

    assert_eq!(value.0, 42);
}

#[test]
fn test_missing_dynamic_field() {
    #[derive(Debug, FromValue)]
    struct ProxyStruct {
        missing: u32,
    }

    assert_vm_error!(
        ProxyStruct => r#"
        fn main() {
            struct Ignored;
            struct Value { other, ignored }
            Value { other: 42, ignored: Ignored }
        }
        "#,
        MissingDynamicStructField { target, name } => {
            assert_eq!(target, "vm_test_from_value_derive::test_missing_dynamic_field::ProxyStruct");
            assert_eq!(name, "missing");
        }
    );

    #[derive(Debug, FromValue)]
    struct ProxyTuple(u32, u32);

    assert_vm_error!(
        ProxyTuple => r#"
        fn main() {
            struct Ignored;
            struct Value(other);
            Value(42)
        }
        "#,
        MissingDynamicStructTupleIndex { target, index } => {
            assert_eq!(target, "vm_test_from_value_derive::test_missing_dynamic_field::ProxyTuple");
            assert_eq!(index, 1);
        }
    );
}
