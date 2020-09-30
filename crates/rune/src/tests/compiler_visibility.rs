use crate::testing::*;

#[test]
fn test_working_visibility() {
    let output = rune!(i64 => r#"
    pub mod a {
        pub struct Foo;
        fn hidden() { 42 }
        pub fn visible() { hidden() }
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
        pub mod a {
            pub struct Foo;
            fn hidden() { 42 }
            pub fn visible() { hidden() }
        }

        fn main() {
            a::hidden()
        }  
        "#,
        span, QueryError { error } => {
            assert_eq!(span, Span::new(165, 174));

            match *error {
                NotVisible { .. } => (),
                other => panic!("unexpected query error: {:?}", other),
            }
        }
    };
}
