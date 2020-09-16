use crate::testing::*;

#[test]
fn test_assign_exprs() {
    assert_parse!(r#"fn main() { let var = 1; var = 42; }"#);

    assert_compile_error! {
        r#"fn main() { 1 = 42; }"#,
        span, UnsupportedAssignExpr => {
            assert_eq!(span, Span::new(12, 18));
        }
    };
}
