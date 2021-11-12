use rune::meta::{CompileMeta, CompileMetaKind};
use rune::CompileErrorKind::*;
use rune::Span;
use rune_tests::*;

#[test]
fn test_use_variant_as_type() {
    assert_compile_error! {
        r#"pub fn main() { Err(0) is Err }"#,
        span, ExpectedMeta { meta: CompileMeta { kind: CompileMetaKind::TupleVariant { .. }, .. }, .. } => {
            assert_eq!(span, Span::new(26, 29));
        }
    };
}

#[test]
fn break_outside_of_loop() {
    assert_compile_error! {
        r#"pub fn main() { break; }"#,
        span, BreakOutsideOfLoop => {
            assert_eq!(span, Span::new(16, 21));
        }
    };
}

#[test]
fn test_pointers() {
    assert_compile_error! {
        r#"pub fn main() { let n = 0; foo(&n); } fn foo(n) {}"#,
        span, UnsupportedRef => {
            assert_eq!(span, Span::new(31, 33));
        }
    };
}

#[test]
fn test_template_strings() {
    assert_parse!(r#"pub fn main() { `hello \`` }"#);
    assert_parse!(r#"pub fn main() { `hello \$` }"#);
}

#[test]
fn test_wrong_arguments() {
    assert_compile_error! {
        r#"pub fn main() { Some(1, 2) }"#,
        span, UnsupportedArgumentCount { expected, actual, .. } => {
            assert_eq!(span, Span::new(16, 26));
            assert_eq!(expected, 1);
            assert_eq!(actual, 2);
        }
    };

    assert_compile_error! {
        r#"pub fn main() { None(1) }"#,
        span, UnsupportedArgumentCount { expected, actual, .. } => {
            assert_eq!(span, Span::new(16, 23));
            assert_eq!(expected, 0);
            assert_eq!(actual, 1);
        }
    };
}

#[test]
fn test_bad_struct_declaration() {
    assert_compile_error! {
        r#"struct Foo { a, b } pub fn main() { Foo { a: 12 } }"#,
        span, LitObjectMissingField { field, .. } => {
            assert_eq!(span, Span::new(36, 49));
            assert_eq!(field.as_ref(), "b");
        }
    };

    assert_compile_error! {
        r#"struct Foo { a, b } pub fn main() { Foo { not_field: 12 } }"#,
        span, LitObjectNotField { field, .. } => {
            assert_eq!(span, Span::new(42, 51));
            assert_eq!(field.as_ref(), "not_field");
        }
    };

    assert_compile_error! {
        r#"pub fn main() { None(1) }"#,
        span, UnsupportedArgumentCount { expected, actual, .. } => {
            assert_eq!(span, Span::new(16, 23));
            assert_eq!(expected, 0);
            assert_eq!(actual, 1);
        }
    };
}
