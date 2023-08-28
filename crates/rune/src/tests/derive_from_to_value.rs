#![allow(
    unused,
    clippy::enum_variant_names,
    clippy::vec_init_then_push,
    clippy::needless_return
)]

prelude!();

#[derive(Any)]
struct Custom {}

#[derive(FromValue)]
struct TestUnit;

#[derive(FromValue)]
struct TestNamed {
    a: Mut<alloc::String>,
    b: Mut<Tuple>,
    c: Mut<Object>,
    d: Ref<Custom>,
    e: Mut<Custom>,
}

#[derive(FromValue)]
struct TestUnnamed(Mut<alloc::String>, Mut<Custom>);

#[derive(ToValue)]
struct Test2 {
    a: alloc::String,
    b: OwnedTuple,
    c: Object,
    d: Custom,
    e: Custom,
}

#[derive(ToValue)]
struct Test2Unnamed(alloc::String, Custom);

#[derive(FromValue)]
enum TestEnum {
    TestUnit,
    TestNamed {
        a: Mut<alloc::String>,
        b: Mut<Tuple>,
        c: Mut<Object>,
        d: Ref<Custom>,
        e: Mut<Custom>,
    },
    TestUnnamed(
        Mut<alloc::String>,
        Mut<Tuple>,
        Mut<Object>,
        Ref<Custom>,
        Mut<Custom>,
    ),
}

#[test]
fn derive_from_to_value() {}
