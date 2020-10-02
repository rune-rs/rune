use crate::testing::*;

#[test]
fn test_working_visibility() {
    let output = rune!(i64 => r#"
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
    "#);

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
    let result = rune! {
        i64 => r#"
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
        "#
    };

    assert_eq!(result, 2);
}
