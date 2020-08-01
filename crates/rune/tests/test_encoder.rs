use rune::CompileError::*;
use st::unit::Span;

macro_rules! test_encode {
    ($source:expr) => {{
        rune::compile($source).unwrap();
    }};
}

macro_rules! test_err {
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

#[test]
fn break_outside_of_loop() {
    test_err! {
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
    test_err! {
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
                *var = 42;
                **var = 42;
            }
        "#
    };

    test_err! {
        UnsupportedAssignExpr { span } => assert_eq!(span, Span::new(41, 46)),
        r#"
            fn main() {
                1 + 1 = 42;
            }
        "#
    };
}

#[test]
fn test_return_local_reference() {
    test_err! {
        ReturnLocalReferences { span, references_at, block } => {
            assert_eq!(span, Span::new(79, 91));
            assert_eq!(references_at, vec![Span::new(88, 90), Span::new(84, 86), Span::new(80, 82)]);
            assert_eq!(block, Span::new(19, 101));
        },
        r#"
        fn foo(n) {
            let v = 5;
            let u = 6;
            [&v, &n, &u]
        }
        "#
    };
}
