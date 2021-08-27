#![allow(
    unused,
    clippy::enum_variant_names,
    clippy::vec_init_then_push,
    clippy::needless_return
)]

use runestick::{Mut, Object, Ref, Shared, Tuple, Value};
use runestick_macros::{Any, FromValue, ToValue};

#[derive(Any)]
struct Custom {}

#[derive(FromValue)]
struct TestUnit;

#[derive(FromValue)]
struct TestNamed {
    a: Mut<String>,
    b: Mut<Tuple>,
    c: Mut<Object>,
    d: Ref<Custom>,
    e: Mut<Custom>,
}

#[derive(FromValue)]
struct TestUnnamed(Mut<String>, Mut<Custom>);

#[derive(ToValue)]
struct Test2 {
    a: String,
    b: Tuple,
    c: Object,
    d: Custom,
    e: Custom,
}

#[derive(ToValue)]
struct Test2Unnamed(String, Custom);

#[derive(FromValue)]
enum TestEnum {
    TestUnit,
    TestNamed {
        a: Mut<String>,
        b: Mut<Tuple>,
        c: Mut<Object>,
        d: Ref<Custom>,
        e: Mut<Custom>,
    },
    TestUnnamed(
        Mut<String>,
        Mut<Tuple>,
        Mut<Object>,
        Ref<Custom>,
        Mut<Custom>,
    ),
}

#[test]
fn test_macro() {}
