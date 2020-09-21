use crate::testing::*;

#[test]
fn test_bad_attributes() {
    assert_compile_error! {
        r#"fn main() { #[foo] #[bar] let x =  1; }"#,
        span, Internal { msg } => {
            assert_eq!(msg,  "expression attributes are not supported");
            assert_eq!(span, Span::new(12, 36));
        }
    };
}
