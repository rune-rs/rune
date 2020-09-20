use crate::testing::*;

#[test]
fn test_visibility_not_supported() {
    assert_compile_error! {
        r#"pub fn main() { 0 }"#,
        span, Internal {..} => {
            assert_eq!(span, Span::new(0, 3));
        }
    };
}
