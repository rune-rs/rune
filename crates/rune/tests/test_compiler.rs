use rune::CompileError::*;
use rune::ParseError::*;
use rune::Warning::*;
use runestick::unit::Span;

macro_rules! test_parse {
    ($source:expr) => {{
        let context = runestick::Context::with_default_packages().unwrap();
        rune::compile(&context, $source).unwrap();
    }};
}

macro_rules! test_compile_error {
    ($source:expr, $pat:pat => $cond:expr) => {{
        let context = runestick::Context::with_default_packages().unwrap();
        let err = rune::compile(&context, $source).unwrap_err();

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
        let context = runestick::Context::with_default_packages().unwrap();
        let (_, warnings) = rune::compile(&context, $source).expect("source should compile");
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
        let context = runestick::Context::with_default_packages().unwrap();
        let err = rune::compile(&context, $source).unwrap_err();

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
fn test_pointers() {
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
fn test_template_strings() {
    test_parse!(r#"fn main() { `hello \}` }"#);

    test_compile_error! {
        r#"fn main() { `hello }` }"#,
        ParseError { error: UnexpectedCloseBrace { span } } => {
            assert_eq!(span, Span::new(13, 20));
        }
    };
}

#[test]
fn test_wrong_arguments() {
    test_compile_error! {
        r#"fn main() { Some(1, 2) }"#,
        UnsupportedArgumentCount { span, expected, actual, .. } => {
            assert_eq!(span, Span::new(12, 22));
            assert_eq!(expected, 1);
            assert_eq!(actual, 2);
        }
    };

    test_compile_error! {
        r#"fn main() { None(1) }"#,
        UnsupportedArgumentCount { span, expected, actual, .. } => {
            assert_eq!(span, Span::new(12, 19));
            assert_eq!(expected, 0);
            assert_eq!(actual, 1);
        }
    };
}

#[test]
fn test_bad_struct_declaration() {
    test_compile_error! {
        r#"struct Foo { a, b } fn main() { Foo { a: 12 } }"#,
        LitObjectMissingField { span, field, .. } => {
            assert_eq!(span, Span::new(32, 45));
            assert_eq!(field, "b");
        }
    };

    test_compile_error! {
        r#"struct Foo { a, b } fn main() { Foo { not_field: 12 } }"#,
        LitObjectNotField { span, field, .. } => {
            assert_eq!(span, Span::new(38, 47));
            assert_eq!(field, "not_field");
        }
    };

    test_compile_error! {
        r#"fn main() { None(1) }"#,
        UnsupportedArgumentCount { span, expected, actual, .. } => {
            assert_eq!(span, Span::new(12, 19));
            assert_eq!(expected, 0);
            assert_eq!(actual, 1);
        }
    };
}

#[test]
fn test_let_pattern_might_panic() {
    test_warnings! {
        r#"fn main() { let [0, 1, 3] = []; }"#,
        LetPatternMightPanic { span, .. } => {
            assert_eq!(span, Span::new(12, 30));
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
fn test_remove_variant_parens() {
    test_warnings! {
        r#"fn main() { None() }"#,
        RemoveTupleCallParams { span, .. } => {
            assert_eq!(span, Span::new(12, 18));
        }
    };
}
