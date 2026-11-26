//! Tests for `const` items declared in script `impl` blocks.

prelude!();

/// An impl constant is an item under the type, so without indexing it there
/// `Foo::X` names nothing.
#[test]
fn an_impl_constant_is_reachable_through_its_type() {
    let out: i64 = eval(
        r#"
        struct Foo;

        impl Foo {
            const X = 10;
        }

        Foo::X
        "#,
    );
    assert_eq!(out, 10);
}

/// The visibility a constant takes at the top level is accepted on one in an
/// impl block as well, rather than rejected the way it is on an impl function.
#[test]
fn an_impl_constant_may_be_public() {
    let out: i64 = eval(
        r#"
        struct Foo;

        impl Foo {
            pub const X = 10;
        }

        Foo::X
        "#,
    );
    assert_eq!(out, 10);
}

/// `Self` inside an impl function names the type, so it reaches the type's
/// constants the same way it reaches its functions.
#[test]
fn an_impl_constant_is_reachable_through_self_in_an_impl_function() {
    let out: i64 = eval(
        r#"
        struct Foo;

        impl Foo {
            const X = 10;

            fn x() { Self::X }
        }

        Foo::x()
        "#,
    );
    assert_eq!(out, 10);
}

/// One impl constant may be defined in terms of another, by `Self` or by the
/// type's name, in either order.
#[test]
fn an_impl_constant_may_refer_to_another() {
    let out: i64 = eval(
        r#"
        struct Foo;

        impl Foo {
            const B = Self::A * 2;
            const A = 21;
            const C = Foo::B + 1;
        }

        Foo::C
        "#,
    );
    assert_eq!(out, 43);
}

/// An impl constant is a constant, so it can be used where one is required.
#[test]
fn an_impl_constant_can_be_used_in_a_pattern() {
    let out: i64 = eval(
        r#"
        struct Foo;

        impl Foo {
            const X = 10;
        }

        match 10 { Foo::X => 1, _ => 0 }
        "#,
    );
    assert_eq!(out, 1);
}

/// Without this a constant and a function of the same name under one type
/// would silently shadow each other. It is reported even when neither is used,
/// the way two top-level items of the same name are.
#[test]
fn an_impl_constant_and_function_of_the_same_name_is_an_error() {
    assert_errors! {
        "struct Foo; impl Foo { const X = 10; fn X() { 1 } } 0",
        span!(33, 35), ErrorKind::AmbiguousItem { item, .. } => {
            assert_eq!(item.to_string(), "Foo::X");
        }
    };
}

/// Two impl blocks for one type share its namespace, so a constant declared in
/// both is as ambiguous as one declared twice in the same block.
#[test]
fn an_impl_constant_declared_in_two_impl_blocks_is_an_error() {
    assert_errors! {
        "struct Foo; impl Foo { const X = 10; } impl Foo { const X = 11; } 0",
        span!(33, 35), ErrorKind::AmbiguousItem { item, .. } => {
            assert_eq!(item.to_string(), "Foo::X");
        }
    };
}

/// A constant which refers to itself cannot be evaluated, and that is a
/// compile error rather than an overflow. It reads as a missing item, which is
/// what a cycle between top-level constants is reported as too.
#[test]
fn an_impl_constant_cycle_is_an_error() {
    assert_errors! {
        "struct Foo; impl Foo { const A = Self::B; const B = Self::A; } 0",
        _, ErrorKind::MissingItem { item } => {
            assert!(matches!(item.to_string().as_str(), "Foo::A" | "Foo::B"), "{item}");
        }
    };
}

/// Constant evaluation cannot build a value of a type defined in a script yet,
/// so the motivating `const RED = Self(255, 0, 0, 255)` is reported as a compile
/// error naming the type rather than accepted. When that changes, this test is
/// the one to turn into `Color::RED.0 == 255`.
#[test]
fn an_impl_constant_cannot_yet_hold_a_script_type() {
    let mut diagnostics = Default::default();

    let result = crate::tests::compile_helper(
        "pub struct Color(a, b, c, d); impl Color { pub const RED = Self(255, 0, 0, 255); } Color::RED.0",
        &mut diagnostics,
    );

    assert!(result.is_err());

    let message = diagnostics
        .into_diagnostics()
        .into_iter()
        .map(|d| format!("{d:?}"))
        .collect::<Vec<_>>();

    assert!(
        message
            .iter()
            .any(|m| m.contains("ConstNotSupported") && m.contains("Color")),
        "{message:?}"
    );
}
