use rune::testing::*;

#[test]
fn test_working_visibility() {
    let output = rune! { i64 =>
        mod a {
            pub struct Foo;

            mod b {
                pub(super) fn hidden() { 42 }
            }

            pub fn visible() { b::hidden() }
        }

        fn main() {
            a::visible()
        }
    };

    assert_eq!(output, 42);
}

#[test]
fn test_access_hidden() {
    assert_compile_error! {
        r#"
        mod a {
            pub struct Foo;

            mod b {
                pub(super) fn hidden() { 42 }
            }

            pub fn visible() { b::hidden() }
        }

        fn main() {
            a::b::hidden()
        }        
        "#,
        span, QueryError { error } => {
            assert_eq!(span, Span::new(215, 227));
            assert_matches!(*error, NotVisibleMod { .. });
        }
    };
}

#[test]
fn test_indirect_access() {
    let result = rune! { i64 =>
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

        fn main() {
            d::e::test().0
        }
    };

    assert_eq!(result, 2);
}

// Test borrowed from: https://doc.rust-lang.org/reference/visibility-and-privacy.html
#[test]
fn test_rust_example() {
    rune! { () =>
        mod crate_helper_module {
            pub fn crate_helper() {}

            fn implementation_detail() {}
        }

        pub fn public_api() {}

        pub mod submodule {
            use crate_helper_module;

            pub fn my_method() {
                crate_helper_module::crate_helper();
            }

            fn my_implementation() {}

            mod test {
                fn test_my_implementation() {
                    super::my_implementation();
                }
            }
        }

        fn main() {
            submodule::my_method();
        }
    };
}
