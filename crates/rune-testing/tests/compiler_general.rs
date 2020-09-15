use rune_testing::*;

#[test]
fn test_use_variant_as_type() {
    assert_compile_error! {
        r#"fn main() { Err(0) is Err }"#,
        span, UnsupportedType { meta: CompileMeta { kind: CompileMetaKind::TupleVariant { .. }, .. } } => {
            assert_eq!(span, Span::new(22, 25));
        }
    };
}

#[test]
fn break_outside_of_loop() {
    assert_compile_error! {
        r#"fn main() { break; }"#,
        span, BreakOutsideOfLoop => {
            assert_eq!(span, Span::new(12, 17));
        }
    };
}

#[test]
fn test_pointers() {
    assert_compile_error! {
        r#"fn main() { let n = 0; foo(&n); }"#,
        span, UnsupportedRef => {
            assert_eq!(span, Span::new(27, 29));
        }
    };
}

#[test]
fn test_template_strings() {
    assert_parse!(r#"fn main() { `hello \}` }"#);

    assert_compile_error! {
        r#"fn main() { `hello }` }"#,
        span, ParseError { error: UnexpectedCloseBrace {} } => {
            assert_eq!(span, Span::new(13, 20));
        }
    };
}

#[test]
fn test_wrong_arguments() {
    assert_compile_error! {
        r#"fn main() { Some(1, 2) }"#,
        span, UnsupportedArgumentCount { expected, actual, .. } => {
            assert_eq!(span, Span::new(12, 22));
            assert_eq!(expected, 1);
            assert_eq!(actual, 2);
        }
    };

    assert_compile_error! {
        r#"fn main() { None(1) }"#,
        span, UnsupportedArgumentCount { expected, actual, .. } => {
            assert_eq!(span, Span::new(12, 19));
            assert_eq!(expected, 0);
            assert_eq!(actual, 1);
        }
    };
}

#[test]
fn test_bad_struct_declaration() {
    assert_compile_error! {
        r#"struct Foo { a, b } fn main() { Foo { a: 12 } }"#,
        span, LitObjectMissingField { field, .. } => {
            assert_eq!(span, Span::new(32, 45));
            assert_eq!(field, "b");
        }
    };

    assert_compile_error! {
        r#"struct Foo { a, b } fn main() { Foo { not_field: 12 } }"#,
        span, LitObjectNotField { field, .. } => {
            assert_eq!(span, Span::new(38, 47));
            assert_eq!(field, "not_field");
        }
    };

    assert_compile_error! {
        r#"fn main() { None(1) }"#,
        span, UnsupportedArgumentCount { expected, actual, .. } => {
            assert_eq!(span, Span::new(12, 19));
            assert_eq!(expected, 0);
            assert_eq!(actual, 1);
        }
    };
}
