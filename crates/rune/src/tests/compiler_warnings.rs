prelude!();

use diagnostics::WarningDiagnosticKind::*;

#[test]
fn test_let_pattern_might_panic() {
    assert_warnings! {
        "let [0, 1, 3] = [];",
        span!(4, 13), LetPatternMightPanic { context: Some(span!(0, 19)), .. }
    };
}

#[test]
fn test_template_without_variables() {
    assert_warnings! {
        "`Hello World`",
        span!(0, 13), TemplateWithoutExpansions { context: Some(span!(0, 13)), .. }
    };
}
