use rune_tests::*;

#[test]
fn test_binary_exprs() {
    assert_parse_error! {
        r#"pub fn main() { 0 < 10 >= 10 }"#,
        span, PrecedenceGroupRequired => {
            assert_eq!(span, Span::new(16, 22));
        }
    };

    // Test solving precedence with groups.
    assert_parse!(r#"pub fn main() { (0 < 10) >= 10 }"#);
    assert_parse!(r#"pub fn main() { 0 < (10 >= 10) }"#);
    assert_parse!(r#"pub fn main() { 0 < 10 && 10 > 0 }"#);
    assert_parse!(r#"pub fn main() { 0 < 10 && 10 > 0 || true }"#);
    assert_parse!(r#"pub fn main() { false || return }"#);
}
