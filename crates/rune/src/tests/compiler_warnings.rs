prelude!();

use diagnostics::WarningDiagnosticKind::*;

#[test]
fn test_let_pattern_might_panic() {
    assert_warnings! {
        r#"pub fn main() { let [0, 1, 3] = []; }"#,
        span!(20, 29), LetPatternMightPanic { context: Some(span!(14, 37)), .. }
    };
}

#[test]
fn test_template_without_variables() {
    assert_warnings! {
        r#"pub fn main() { `Hello World` }"#,
        span!(16, 29), TemplateWithoutExpansions { context: Some(span!(14, 31)), .. }
    };
}
