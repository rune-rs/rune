prelude!();

use ErrorKind::*;

#[test]
fn test_binary_exprs() {
    assert_errors! {
        r#"pub fn main() { 0 < 10 >= 10 }"#,
        span, PrecedenceGroupRequired => {
            assert_eq!(span, span!(16, 22));
        }
    };

    // Test solving precedence with groups.
    assert_parse!(r#"pub fn main() { (0 < 10) >= 10 }"#);
    assert_parse!(r#"pub fn main() { 0 < (10 >= 10) }"#);
    assert_parse!(r#"pub fn main() { 0 < 10 && 10 > 0 }"#);
    assert_parse!(r#"pub fn main() { 0 < 10 && 10 > 0 || true }"#);
    assert_parse!(r#"pub fn main() { false || return }"#);
}

#[test]
fn test_basic_operator_precedence() {
    let result: bool = rune! {
        pub fn main() {
            10 < 5 + 10 && 5 > 4
        }
    };

    assert!(result);

    let result: bool = rune! {
        pub fn main() {
            10 < 5 - 10 && 5 > 4
        }
    };

    assert!(!result);
}
