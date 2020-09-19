use crate::testing::*;

#[test]
fn test_bad_attributes() {
    assert_parse_error! {
        r#"fn main() { #[foo] #[bar] 42 }"#,
        span, AttributesNotSupported => {
            assert_eq!(span, Span::new(12, 25));
        }
    };
}
