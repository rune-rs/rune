prelude!();

use ErrorKind::*;

#[test]
fn assign_expr() {
    assert_parse!(r#"pub fn main() { let var = 1; var = 42; }"#);

    assert_errors! {
        r#"pub fn main() { 1 = 42; }"#,
        span!(16, 22), UnsupportedAssignExpr
    };
}

#[test]
fn mut_let() {
    assert_errors! {
        r#"pub fn main() { let mut var = 1; }"#,
        span!(20, 23), UnsupportedMut
    };
}
