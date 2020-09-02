use rune_testing::*;

#[test]
fn test_let_pattern_might_panic() {
    assert_warnings! {
        r#"fn main() { let [0, 1, 3] = []; }"#,
        LetPatternMightPanic { span, .. } => {
            assert_eq!(span, Span::new(12, 30));
        }
    };
}

#[test]
fn test_break_as_value() {
    assert_warnings! {
        r#"fn main() { loop { let _ = break; } }"#,
        BreakDoesNotProduceValue { span, .. } => {
            assert_eq!(span, Span::new(27, 32));
        }
    };
}

#[test]
fn test_template_without_variables() {
    assert_warnings! {
        r#"fn main() { `Hello World` }"#,
        TemplateWithoutExpansions { span, .. } => {
            assert_eq!(span, Span::new(12, 25));
        }
    };
}

#[test]
fn test_remove_variant_parens() {
    assert_warnings! {
        r#"fn main() { None() }"#,
        RemoveTupleCallParams { span, .. } => {
            assert_eq!(span, Span::new(12, 18));
        }
    };
}
