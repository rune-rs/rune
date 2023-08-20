prelude!();

use VmErrorKind::*;

#[test]
fn test_from_value_object_like() {
    #[derive(FromValue)]
    struct Proxy {
        field: u32,
    }

    let value: Proxy = rune! {
        struct Ignored;
        struct Value { field, ignored }
        pub fn main() { Value { field: 42, ignored: Ignored } }
    };

    assert_eq!(value.field, 42);

    let value: Proxy = rune! {
        struct Ignored;
        pub fn main() { #{ field: 42, ignored: Ignored } }
    };

    assert_eq!(value.field, 42);
}

#[test]
fn test_from_value_tuple_like() {
    #[derive(FromValue)]
    struct Proxy(u32);

    let value: Proxy = rune! {
        struct Value(field);
        pub fn main() { Value(42) }
    };

    assert_eq!(value.0, 42);

    let value: Proxy = rune! {
        pub fn main() { (42,) }
    };

    assert_eq!(value.0, 42);
}

#[test]
fn test_missing_dynamic_field() {
    #[derive(Debug, FromValue)]
    struct ProxyStruct {
        #[allow(dead_code)]
        missing: u32,
    }

    assert_vm_error!(
        ProxyStruct => r#"
        pub fn main() {
            struct Ignored;
            struct Value { other, ignored }
            Value { other: 42, ignored: Ignored }
        }
        "#,
        MissingStructField { target, name } => {
            assert!(target.ends_with("::test_missing_dynamic_field::ProxyStruct"));
            assert_eq!(name, "missing");
        }
    );

    #[derive(Debug, FromValue)]
    struct ProxyTuple(u32, u32);

    assert_vm_error!(
        ProxyTuple => r#"
        pub fn main() {
            struct Ignored;
            struct Value(other);
            Value(42)
        }
        "#,
        MissingTupleIndex { target, index } => {
            assert!(target.ends_with("::test_missing_dynamic_field::ProxyTuple"));
            assert_eq!(index, 1);
        }
    );
}

#[test]
fn test_enum_proxy() {
    #[derive(Debug, PartialEq, Eq, FromValue)]
    enum Proxy {
        Empty,
        Tuple(String),
        Struct { field: String },
    }

    let proxy: Proxy = rune! {
        pub fn main() {
            enum Proxy { Empty, Tuple(a), Struct { field } }
            Proxy::Empty
        }
    };

    assert_eq!(proxy, Proxy::Empty);

    let proxy: Proxy = rune! {
        pub fn main() {
            enum Proxy { Empty, Tuple(a), Struct { field } }
            Proxy::Tuple("Hello World")
        }
    };

    assert_eq!(proxy, Proxy::Tuple(String::from("Hello World")));

    let proxy: Proxy = rune! {
        pub fn main() {
            enum Proxy { Empty, Tuple(a), Struct { field } }
            Proxy::Struct { field: "Hello World" }
        }
    };

    assert_eq!(
        proxy,
        Proxy::Struct {
            field: String::from("Hello World")
        }
    );
}
