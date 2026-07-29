prelude!();

use ErrorKind::*;

#[test]
fn test_super_self_crate_mod() {
    let out: i64 = rune! {
        struct Foo;

        impl Foo {
            fn foo() { Self::bar() ^ 0b100000 }

            fn bar() { self::a::foo() ^ 0b10000 }
        }

        pub mod a {
            pub fn foo() { self::b::foo() ^ 0b1000 }

            pub mod b {
                pub fn foo() { super::c::foo() ^ 0b100 }
            }

            pub mod c {
                pub fn foo() { crate::root() ^ 0b10 }
            }
        }

        fn root() { 0b1 }

        Foo::foo()
    };
    assert_eq!(out, 0b111111);
}

#[test]
fn test_super_use() {
    let out: i64 = rune! {
        pub mod x {
            pub mod y {
                pub fn foo() {
                    use crate::VALUE as A;
                    use super::VALUE as B;
                    A + B
                }
            }

            const VALUE = 2;
        }

        const VALUE = 1;

        x::y::foo()
    };
    assert_eq!(out, 3);
}

#[test]
fn test_unsupported_leading_path() {
    assert_errors! {
        r#"use foo::crate::bar;"#,
        span!(9, 14), ExpectedLeadingPathSegment
    };

    assert_errors! {
        r#"use foo::{bar::crate, baz};"#,
        span!(15, 20), ExpectedLeadingPathSegment
    };
}

#[test]
fn test_import_conflict() {
    assert_errors! {
        r#"use std::{option, option};"#,
        span!(10, 16), AmbiguousItem { .. }
    };
}

/// A generic argument which is itself generic ends on `>>`, which the lexer
/// produces as a shift, so the parser splits it to close both lists.
///
/// Without the split neither list closes and the shift is parsed as an operator
/// applied to the unclosed path, which turns the source into a different
/// program rather than reporting anything.
#[test]
fn nested_generic_arguments_close() {
    // Reported for the path, rather than the source being parsed as a shift of
    // `Vec::<Vec::<i64` by `::new()`.
    // The inner list resolved to a type of its own, so both of them closed.
    assert_errors! {
        "let a = Vec::<Vec::<i64>>::new();",
        span!(14, 24), MissingItemParameters { parameters, .. } => {
            assert_eq!(parameters.len(), 2);
            assert!(parameters[0].is_some());
        }
    }

    // A shift which is not closing a list is untouched.
    let out: i64 = rune!(
        let a = 8 >> 2;
        let b = 1 << 3;
        let c = 16;
        a + b + c
    );

    assert_eq!(out, 26);
}

/// An `impl` block puts what it declares under the type it is for, so a block
/// written in one module for a type from another declares items which are not
/// under the module the block is written in.
///
/// The name of an item was looked for by walking outwards from the item it was
/// written in until the module it belonged to was left behind, which for these
/// blocks never held to begin with. Nothing was searched at all, and the walk
/// asserted that it was starting somewhere it never started.
#[test]
fn names_resolve_in_an_impl_for_another_module() {
    // What the block declares is in scope, including through a nested item.
    let out: i64 = rune! {
        struct Foo;

        mod m {
            impl super::Foo {
                fn nested(self) {
                    struct Bar;

                    impl Bar {
                        fn get(self) { 7 }
                    }

                    Bar.get()
                }

                fn local(self) {
                    let x = 3;
                    x
                }

                fn sibling(self) { Self::helper() }

                fn helper() { 11 }
            }
        }

        Foo.nested() + Foo.local() + Foo.sibling()
    };
    assert_eq!(out, 21);

    // The module the block is written in is in scope, and is what a name
    // resolves to rather than the module the type came from.
    let out: i64 = rune! {
        struct Foo;

        fn helper() { 9 }

        mod m {
            fn helper() { 5 }

            impl super::Foo {
                fn call(self) { helper() }
            }
        }

        Foo.call()
    };
    assert_eq!(out, 5);

    // Which leaves the one the type came from out of scope, as it is for
    // anything else written in that module.
    assert_errors! {
        r#"
        struct Foo;
        fn helper() { 9 }
        mod m {
            impl super::Foo {
                fn call(self) { helper() }
            }
        }
        Foo.call()
        "#,
        _span, MissingItem { item } => {
            assert_eq!(item.to_string(), "m::helper");
        }
    };

    // What the block declares is searched before the module it is written in,
    // since it is the closer of the two.
    let out: String = rune! {
        struct Foo;

        mod m {
            fn which() { "module" }

            impl super::Foo {
                fn call(self) { which() }

                fn which() { "impl" }
            }
        }

        Foo.call()
    };
    assert_eq!(out, "impl");
}
