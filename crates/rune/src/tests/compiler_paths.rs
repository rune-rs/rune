use crate::testing::*;

#[test]
fn test_super_self_crate_mod() {
    assert_eq! {
        rune! { i64 =>
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

            fn main() { Foo::foo() }
        },
        0b111111,
    };
}

#[test]
fn test_super_use() {
    assert_eq! {
        rune! { i64 =>
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

            fn main() { x::y::foo() }
        },
        3,
    };
}

#[test]
fn test_unsupported_leading_path() {
    assert_compile_error! {
        r#"use foo::crate::bar;"#,
        span, ExpectedLeadingPathSegment => {
            assert_eq!(span, Span::new(9, 14));
        }
    };

    assert_compile_error! {
        r#"use foo::{bar::crate, baz};"#,
        span, ExpectedLeadingPathSegment => {
            assert_eq!(span, Span::new(15, 20));
        }
    };
}

#[test]
fn test_import_conflict() {
    assert_compile_error! {
        r#"use std::{option, option};"#,
        span, QueryError { error, .. } => {
            assert_eq!(span, Span::new(18, 24));
            assert_matches!(&*error, ItemConflict { .. });
        }
    };
}
