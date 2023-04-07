use rune_tests::prelude::*;

use diagnostics::WarningDiagnosticKind::*;

#[test]
fn test_let_pattern_might_panic() {
    assert_warnings! {
        r#"pub fn main() { let [0, 1, 3] = []; }"#,
        LetPatternMightPanic { span, .. } => {
            assert_eq!(span, span!(16, 35));
        }
    };
}

#[test]
fn test_template_without_variables() {
    assert_warnings! {
        r#"pub fn main() { `Hello World` }"#,
        TemplateWithoutExpansions { span, .. } => {
            assert_eq!(span, span!(16, 29));
        }
    };
}

#[test]
fn test_remove_variant_parens() {
    assert_warnings! {
        r#"pub fn main() { None() }"#,
        RemoveTupleCallParams { span, .. } => {
            assert_eq!(span, span!(16, 22));
        }
    };
}
