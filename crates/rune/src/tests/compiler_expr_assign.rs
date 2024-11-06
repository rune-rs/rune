prelude!();

use ErrorKind::*;

#[test]
fn assign_expr() {
    assert_parse!(r#"let var = 1; var = 42;"#);

    assert_errors! {
        r#"1 = 42;"#,
        span!(0, 6), UnsupportedAssignExpr
    };
}

#[test]
fn mut_let() {
    assert_errors! {
        r#"let mut var = 1;"#,
        span!(4, 7), UnsupportedMut
    };
}
