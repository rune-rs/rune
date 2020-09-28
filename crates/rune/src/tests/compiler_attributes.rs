use crate::testing::*;

#[test]
fn test_bad_attributes() {
    assert_compile_error! {
        r#"fn main() { #[foo] #[bar] let x = 1; }"#,
        span, Internal { message } => {
            assert_eq!(message, "attributes are not supported");
            assert_eq!(span, Span::new(12, 25));
        }
    };
}
