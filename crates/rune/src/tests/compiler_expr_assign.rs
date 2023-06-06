prelude!();

use CompileErrorKind::*;

#[test]
fn test_assign_exprs() {
    assert_parse!(r#"pub fn main() { let var = 1; var = 42; }"#);

    assert_errors! {
        r#"pub fn main() { 1 = 42; }"#,
        span!(16, 22), UnsupportedAssignExpr
    };
}
