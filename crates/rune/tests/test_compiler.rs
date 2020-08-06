use rune::CompileError::*;
use rune::ParseError::*;
use rune::Warning::*;
use stk::unit::Span;

macro_rules! test_parse {
    ($source:expr) => {{
        rune::compile($source).unwrap();
    }};
}

macro_rules! test_compile_error {
    ($source:expr, $pat:pat => $cond:expr) => {{
        let err = rune::compile($source).unwrap_err();

        match err {
            rune::Error::CompileError($pat) => ($cond),
            _ => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), err);
            }
        }
    }};
}

macro_rules! test_warnings {
    ($source:expr $(, $pat:pat => $cond:expr)*) => {{
        let (_, warnings) = rune::compile($source).expect("source should compile");
        assert!(!warnings.is_empty(), "no warnings produced");

        let mut it = warnings.into_iter();

        $(
            let warning = it.next().expect("expected a warning");

            match warning {
                $pat => ($cond),
                warning => {
                    panic!("expected warning `{}` but was `{:?}`", stringify!($pat), warning);
                }
            }
        )*

        assert!(it.next().is_none(), "there should be no more warnings");
    }};
}

macro_rules! test_parse_error {
    ($source:expr, $pat:pat => $cond:expr) => {{
        let err = rune::compile($source).unwrap_err();

        match err {
            rune::Error::ParseError($pat) => ($cond),
            _ => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), err);
            }
        }
    }};
}

#[test]
fn break_outside_of_loop() {
    test_compile_error! {
        r#"fn main() { break; }"#,
        BreakOutsideOfLoop { span } => {
            assert_eq!(span, Span::new(12, 17));
        }
    };
}

#[test]
fn test_break_as_value() {
    test_warnings! {
        r#"fn main() { loop { let _ = break; } }"#,
        BreakDoesNotProduceValue { span, .. } => {
            assert_eq!(span, Span::new(27, 32));
        }
    };

    test_warnings! {
        r#"fn main() { loop { break } }"#,
        NotUsed { span, .. } => {
            assert_eq!(span, Span::new(19, 24));
        }
    };
}

#[test]
fn test_assign_exprs() {
    test_parse!(r#"fn main() { let var = 1; var = 42; }"#);

    test_compile_error! {
        r#"fn main() { 1 = 42; }"#,
        UnsupportedAssignExpr { span } => {
            assert_eq!(span, Span::new(12, 18));
        }
    };
}

#[test]
fn test_match() {
    test_parse_error! {
        r#"fn main(n) { match n { _ => 1, _ => 2, } }"#,
        MatchMultipleFallbackBranches { span, existing } => {
            assert_eq!(span, Span::new(31, 37));
            assert_eq!(existing, Span::new(23, 29));
        }
    };

    test_parse_error! {
        r#"fn main(n) { match n { _ => 1, 5 => 2, } }"#,
        MatchNeverReached { span, existing } => {
            assert_eq!(span, Span::new(31, 37));
            assert_eq!(existing, Span::new(23, 29));
        }
    };
}

#[tokio::test]
async fn test_pointers() {
    test_compile_error! {
        r#"fn main() { let n = 0; foo(&n); }"#,
        UnsupportedRef { span } => {
            assert_eq!(span, Span::new(28, 29));
        }
    };
}

#[test]
fn test_binary_exprs() {
    test_parse_error! {
        r#"fn main() { 0 < 10 >= 10 }"#,
        PrecedenceGroupRequired { span } => {
            assert_eq!(span, Span::new(12, 18));
        }
    };

    // Test solving precedence with groups.
    test_parse!(r#"fn main() { (0 < 10) >= 10 }"#);
    test_parse!(r#"fn main() { 0 < (10 >= 10) }"#);
    test_parse!(r#"fn main() { 0 < 10 && 10 > 0 }"#);
    test_parse!(r#"fn main() { 0 < 10 && 10 > 0 || true }"#);
}

#[test]
fn test_template_without_variables() {
    test_warnings! {
        r#"fn main() { `Hello World` }"#,
        TemplateWithoutExpansions { span, .. } => {
            assert_eq!(span, Span::new(12, 25));
        }
    };
}

#[test]
fn test_template_strings() {
    test_parse!(r#"fn main() { `hello \}` }"#);

    test_compile_error! {
        r#"fn main() { `hello }` }"#,
        ParseError { error: UnexpectedCloseBrace { span } } => {
            assert_eq!(span, Span::new(13, 20));
        }
    };
}
