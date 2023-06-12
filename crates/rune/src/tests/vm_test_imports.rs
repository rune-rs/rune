prelude!();

use ErrorKind::*;

#[test]
fn test_grouped_imports() {
    let out: (i64, bool, bool) = rune! {
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
    };
    assert_eq!(out, (2, true, true));
}

#[test]
fn test_reexport() {
    let out: i64 = rune! {
        mod inner { pub fn func() { 42 } }
        pub use self::inner::func as main;
    };
    assert_eq!(out, 42);

    let out: i64 = rune! {
        mod inner { pub fn func() { 42 } }
        pub use crate::inner::func as main;
    };
    assert_eq!(out, 42);

    let out: i64 = rune! {
        mod inner2 { pub fn func() { 42 } }
        mod inner1 { pub use super::inner2::func; }
        pub use crate::inner1::func as main;
    };

    assert_eq!(out, 42);
}

#[test]
fn test_access() {
    assert!(rune! {
        mod a { pub struct Foo; }

        mod b {
            use c::Foo;
            use crate::a as c;
            pub fn test() { Foo is c::Foo }
        }

        pub fn main() { b::test() }
    });

    assert_errors! {
        r#"
        mod a { struct Test; }
        mod c { use a; fn test() { a::Test } }
        pub fn main() { c::test() }
        "#,
        span!(103, 110), NotVisible { .. }
    };
}
