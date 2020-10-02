use crate::testing::*;
use runestick::FromValue;

#[test]
fn test_from_value_object_like() {
    #[derive(FromValue)]
    struct Proxy {
        field: u32,
    }

    let value = rune! { Proxy =>
        struct Ignored;
        struct Value { field, ignored }
        fn main() { Value { field: 42, ignored: Ignored } }
    };

    assert_eq!(value.field, 42);

    let value = rune! { Proxy =>
        struct Ignored;
        fn main() { #{ field: 42, ignored: Ignored } }
    };

    assert_eq!(value.field, 42);
}

#[test]
fn test_from_value_tuple_like() {
    #[derive(FromValue)]
    struct Proxy(u32);

    let value = rune! { Proxy =>
        struct Value(field);
        fn main() { Value(42) }
    };

    assert_eq!(value.0, 42);

    let value = rune! { Proxy =>
        fn main() { (42,) }
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
        MissingStructField { target, name } => {
            assert_eq!(target, "rune::tests::vm_test_from_value_derive::test_missing_dynamic_field::ProxyStruct");
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
        MissingTupleIndex { target, index } => {
            assert_eq!(target, "rune::tests::vm_test_from_value_derive::test_missing_dynamic_field::ProxyTuple");
            assert_eq!(index, 1);
        }
    );
}

#[test]
fn test_enum_proxy() {
    #[derive(Debug, PartialEq, Eq, FromValue)]
    enum Proxy {
        Unit,
        Tuple(String),
        Struct { field: String },
    }

    let proxy = rune! { Proxy =>
    fn main() {
        enum Proxy { Unit, Tuple(a), Struct { field } }
        Proxy::Unit
    }};

    assert_eq!(proxy, Proxy::Unit);

    let proxy = rune! { Proxy =>
    fn main() {
        enum Proxy { Unit, Tuple(a), Struct { field } }
        Proxy::Tuple("Hello World")
    }};

    assert_eq!(proxy, Proxy::Tuple(String::from("Hello World")));

    let proxy = rune! { Proxy =>
    fn main() {
        enum Proxy { Unit, Tuple(a), Struct { field } }
        Proxy::Struct { field: "Hello World" }
    }};

    assert_eq!(
        proxy,
        Proxy::Struct {
            field: String::from("Hello World")
        }
    );
}
