prelude!();

use diagnostics::WarningDiagnosticKind::*;

#[test]
fn test_let_pattern_might_panic() {
    assert_warnings! {
        r#"pub fn main() { let [0, 1, 3] = []; }"#,
        span!(16, 35), LetPatternMightPanic { context: Some(span!(14, 37)), .. }
    };
}

#[test]
fn test_template_without_variables() {
    assert_warnings! {
        r#"pub fn main() { `Hello World` }"#,
        span!(16, 29), TemplateWithoutExpansions { context: Some(span!(14, 31)), .. }
    };
}

#[test]
fn test_remove_variant_parens() {
    assert_warnings! {
        r#"pub fn main() { None() }"#,
        span!(20, 22), RemoveTupleCallParams { variant: span!(16, 20), .. }
    };
}
