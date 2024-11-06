prelude!();

use ErrorKind::*;

#[test]
fn test_binary_exprs() {
    assert_errors! {
        r#"0 < 10 >= 10"#,
        span!(0, 6), PrecedenceGroupRequired => {
        }
    };

    // Test solving precedence with groups.
    assert_parse!(r#"(0 < 10) >= 10"#);
    assert_parse!(r#"0 < (10 >= 10)"#);
    assert_parse!(r#"0 < 10 && 10 > 0"#);
    assert_parse!(r#"0 < 10 && 10 > 0 || true"#);
    assert_parse!(r#"false || return"#);
}

#[test]
fn test_basic_operator_precedence() {
    let result: bool = rune!(10 < 5 + 10 && 5 > 4);
    assert!(result);

    let result: bool = rune!(10 < 5 - 10 && 5 > 4);
    assert!(!result);
}
