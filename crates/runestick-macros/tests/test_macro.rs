#![allow(unused)]

use runestick::{Mut, Object, Ref, Shared, Tuple, Value};
use runestick_macros::{Any, FromValue, ToValue};

#[derive(Any)]
struct Custom {}

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

#[test]
fn test_macro() {}
