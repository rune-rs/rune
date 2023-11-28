prelude!();

use ErrorKind::*;

#[test]
fn test_working_visibility() {
    let output: i64 = rune! {
        mod a {
            pub struct Foo;

            mod b {
                pub(super) fn hidden() { 42 }
            }

            pub fn visible() { b::hidden() }
        }

        pub fn main() {
            a::visible()
        }
    };

    assert_eq!(output, 42);
}

#[test]
fn test_access_hidden() {
    assert_errors! {
        r#"
        mod a {
            pub struct Foo;

            mod b {
                pub(super) fn hidden() { 42 }
            }

            pub fn visible() { b::hidden() }
        }

        pub fn main() {
            a::b::hidden()
        }        
        "#,
        span, NotVisibleMod { .. } => {
            assert_eq!(span, span!(219, 231));
        }
    };
}

#[test]
fn test_hidden_reexport() {
    assert_errors! {
        r#"
        mod a { struct Foo; }

        mod b {
            use crate::a::Foo;
            pub fn test() { Foo }
        }

        pub fn main() { b::test() }
        "#,
        span, NotVisible { .. } => {
            assert_eq!(span, span!(107, 110));
        }
    }
}

#[test]
fn test_indirect_access() {
    let result: i64 = rune! {
        mod d {
            mod a {
                pub(super) mod b {
                    pub(crate) mod c {
                        pub struct Foo(n);
                    }
                }
            }

            pub mod e {
                pub(crate) fn test() {
                    crate::d::a::b::c::Foo(2)
                }
            }
        }

        pub fn main() {
            d::e::test().0
        }
    };

    assert_eq!(result, 2);
}

// Test borrowed from: https://doc.rust-lang.org/reference/visibility-and-privacy.html
#[test]
fn test_rust_example() {
    rune! {
        mod crate_helper_module {
            pub fn crate_helper() {}

            fn implementation_detail() {}
        }

        pub fn public_api() {}

        pub mod submodule {
            pub fn my_method() {
                crate::crate_helper_module::crate_helper();
            }

            fn my_implementation() {}

            mod test {
                fn test_my_implementation() {
                    super::my_implementation();
                }
            }
        }

        pub fn main() {
            submodule::my_method();
        }
    };
}

#[test]
fn test_access_super() {
    let value: i64 = rune! {
        struct Test;

        mod c {
            pub fn test() { let _ = super::Test; 1 }
        }

        pub fn main() {
            c::test()
        }
    };

    assert_eq!(value, 1);

    let value: i64 = rune! {
        mod a { pub(super) fn test() { 1 } }
        mod b { pub fn test() { crate::a::test() } }

        pub fn main() {
            b::test()
        }
    };

    assert_eq!(value, 1);
}
