use crate::testing::*;

#[test]
fn test_working_visibility() {
    let output = rune!(i64 => r#"
    mod a {
        pub struct Foo;
    
        mod b {
            fn hidden() { 42 }
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
                fn hidden() { 42 }
            }
        
            pub fn visible() { b::hidden() }
        }
        
        fn main() {
            a::b::hidden()
        }        
        "#,
        span, QueryError { error } => {
            assert_eq!(span, Span::new(228, 240));

            match *error {
                NotVisibleMod { .. } => (),
                other => panic!("unexpected query error: {:?}", other),
            }
        }
    };
}
