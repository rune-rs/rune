#![allow(unused)]

use runestick::{Object, OwnedMut, OwnedRef, Shared, Tuple, Value};
use runestick_macros::{Any, FromValue, ToValue};

#[derive(Any)]
struct Custom {}

#[derive(FromValue)]
struct TestNamed {
    a: OwnedMut<String>,
    b: OwnedMut<Tuple>,
    c: OwnedMut<Object>,
    d: OwnedRef<Custom>,
    e: OwnedMut<Custom>,
}

#[derive(FromValue)]
struct TestUnnamed(OwnedMut<String>, OwnedMut<Custom>);

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
