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
    #[rune(any)]
    d: OwnedRef<Custom>,
    #[rune(any)]
    e: OwnedMut<Custom>,
}

#[derive(FromValue)]
struct TestUnnamed(OwnedMut<String>, #[rune(any)] OwnedMut<Custom>);

#[derive(ToValue)]
struct Test2 {
    a: String,
    b: Tuple,
    c: Object,
    #[rune(any)]
    d: Custom,
    #[rune(any)]
    e: Custom,
}

#[derive(ToValue)]
struct Test2Unnamed(String, #[rune(any)] Custom);

#[test]
fn test_macro() {}
