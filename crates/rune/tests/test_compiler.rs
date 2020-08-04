use rune::CompileError::*;
use rune::ParseError::*;
use st::unit::Span;

macro_rules! test_encode {
    ($source:expr) => {{
        rune::compile($source).unwrap();
    }};
}

macro_rules! test_compile_err {
    ($pat:pat => $cond:expr, $source:expr) => {{
        let err = rune::compile($source).unwrap_err();

        match err {
            rune::Error::CompileError($pat) => ($cond),
            _ => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), err);
            }
        }
    }};
}

macro_rules! test_parse_err {
    ($pat:pat => $cond:expr, $source:expr) => {{
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
    test_compile_err! {
        BreakOutsideOfLoop { span } => assert_eq!(span, Span::new(41, 46)),
        r#"
            fn main() {
                break;
            }
        "#
    };
}

#[test]
fn test_break_as_value() {
    test_compile_err! {
        BreakDoesNotProduceValue { span } => assert_eq!(span, Span::new(41, 46)),
        r#"
            fn main() {
                break
            }
        "#
    };
}

#[test]
fn test_assign_exprs() {
    test_encode! {
        r#"
            fn main() {
                let var = 1;
                var = 42;
            }
        "#
    };

    test_compile_err! {
        UnsupportedAssignExpr { span } => assert_eq!(span, Span::new(41, 51)),
        r#"
            fn main() {
                1 + 1 = 42;
            }
        "#
    };
}

#[test]
fn test_match() {
    test_parse_err! {
        MatchMultipleFallbackBranches { span, existing } => {
            assert_eq!(span, Span::new(84, 90));
            assert_eq!(existing, Span::new(60, 66));
        },
        r#"
        fn main(n) {
            match n {
                _ => 1,
                _ => 2,
            }
        }
        "#
    };

    test_parse_err! {
        MatchNeverReached { span, existing } => {
            assert_eq!(span, Span::new(84, 90));
            assert_eq!(existing, Span::new(60, 66));
        },
        r#"
        fn main(n) {
            match n {
                _ => 1,
                5 => 2,
            }
        }
        "#
    };
}

#[tokio::test]
async fn test_pointers() {
    test_compile_err! {
        UnsupportedRef { span } => assert_eq!(span, Span::new(61, 62)),
        r#"
        fn main() {
            let n = 0;
            foo(&n);
        }
        "#
    };
}
