use rune_testing::*;

#[test]
fn test_binary_exprs() {
    assert_parse_error! {
        r#"fn main() { 0 < 10 >= 10 }"#,
        span, PrecedenceGroupRequired => {
            assert_eq!(span, Span::new(12, 18));
        }
    };

    // Test solving precedence with groups.
    assert_parse!(r#"fn main() { (0 < 10) >= 10 }"#);
    assert_parse!(r#"fn main() { 0 < (10 >= 10) }"#);
    assert_parse!(r#"fn main() { 0 < 10 && 10 > 0 }"#);
    assert_parse!(r#"fn main() { 0 < 10 && 10 > 0 || true }"#);
}
