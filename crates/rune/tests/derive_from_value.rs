#![allow(unused)]

use rune::alloc::prelude::*;
use rune::runtime::{Mut, Object, OwnedTuple, Ref, Tuple};
use rune::{Any, FromValue, ToValue};

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
    b: OwnedTuple,
    c: Object,
    d: Custom,
    e: Custom,
}

#[derive(ToValue)]
struct Test2Unnamed(String, Custom);

#[derive(FromValue)]
enum TestEnum {
    Unit,
    Named {
        a: Mut<String>,
        b: Mut<Tuple>,
        c: Mut<Object>,
        d: Ref<Custom>,
        e: Mut<Custom>,
    },
    Unnamed(
        Mut<String>,
        Mut<Tuple>,
        Mut<Object>,
        Ref<Custom>,
        Mut<Custom>,
    ),
}

#[test]
fn derive_from_to_value() {}
