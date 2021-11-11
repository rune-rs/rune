use rune::CompileErrorKind;
use rune::QueryErrorKind::*;
use rune::Span;
use rune_tests::*;

#[test]
fn test_grouped_imports() {
    assert_eq! {
        rune! { (i64, bool, bool) =>
            use a::{b::*, b::Foo::Baz, c};

            pub mod a {
                pub mod b {
                    pub enum Foo { Bar, Baz, }
                }

                pub mod c {
                    pub const VALUE = 2;
                }
            }

            pub fn main() {
                (c::VALUE, Foo::Bar is a::b::Foo, Baz is a::b::Foo)
            }
        },
        (2, true, true),
    };
}

#[test]
fn test_reexport() {
    assert_eq! {
        rune! { i64 =>
            mod inner { pub fn func() { 42 } }
            pub use self::inner::func as main;
        },
        42,
    };

    assert_eq! {
        rune! { i64 =>
            mod inner { pub fn func() { 42 } }
            pub use crate::inner::func as main;
        },
        42,
    };

    assert_eq! {
        rune! { i64 =>
            mod inner2 { pub fn func() { 42 } }
            mod inner1 { pub use super::inner2::func; }
            pub use crate::inner1::func as main;
        },
        42,
    };
}

#[test]
fn test_access() {
    assert!(rune! { bool =>
        mod a { pub struct Foo; }

        mod b {
            use c::Foo;
            use crate::a as c;
            pub fn test() { Foo is c::Foo }
        }

        pub fn main() { b::test() }
    });

    assert_compile_error! {
        r#"
        mod a { struct Test; }
        mod c { use a; fn test() { a::Test } }
        pub fn main() { c::test() }
        "#,
        span, CompileErrorKind::QueryError { error: NotVisible { .. } } => {
            assert_eq!(span, Span::new(103, 110));
        }
    };
}
