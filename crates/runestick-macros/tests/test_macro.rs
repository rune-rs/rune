#![allow(unused)]

use runestick::{Object, OwnedMut, OwnedRef, Tuple, Value};
use runestick_macros::{Any, FromValue};

#[derive(Any)]
struct Custom {}

#[derive(FromValue)]
struct TestNamed {
    a: OwnedMut<String>,
    b: OwnedMut<Tuple>,
    c: OwnedMut<Object<Value>>,
    #[rune(any)]
    d: OwnedRef<Custom>,
    #[rune(any)]
    e: OwnedMut<Custom>,
}

#[derive(FromValue)]
struct TestUnnamed(OwnedMut<String>, #[rune(any)] OwnedMut<Custom>);

#[test]
fn test_macro() {}
