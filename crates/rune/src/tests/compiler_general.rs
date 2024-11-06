prelude!();

use ErrorKind::*;

#[test]
fn test_use_variant_as_type() {
    assert_errors! {
        "Err(0) is Err",
        span!(10, 13), ExpectedMeta { meta, .. } => {
            assert_eq!(meta.to_string(), "variant ::std::result::Result::Err");
        }
    };
}

#[test]
fn break_outside_of_loop() {
    assert_errors! {
        "break;",
        span!(0, 5), BreakUnsupported
    };
}

#[test]
fn for_break_with_value() {
    assert_errors! {
        "for _ in 0..10 { break 42; }",
        span!(17, 25), BreakUnsupportedValue
    };
}

#[test]
fn continue_outside_of_loop() {
    assert_errors! {
        "continue;",
        span!(0, 8), ContinueUnsupported
    };
}

#[test]
fn test_pointers() {
    assert_errors! {
        "let n = 0; foo(&n); fn foo(n) {}",
        span!(15, 17), UnsupportedRef
    };
}

#[test]
fn test_template_strings() {
    assert_parse!(r"`hello \``");
    assert_parse!(r"`hello \$`");
}

#[test]
fn test_wrong_arguments() {
    assert_errors! {
        "Some(1, 2)",
        span!(4, 10), BadArgumentCount { expected: 1, actual: 2, .. }
    };

    assert_errors! {
        "None(1)",
        span!(4, 7), BadArgumentCount { expected: 0, actual: 1, .. }
    };
}

#[test]
fn test_bad_struct_declaration() {
    assert_errors! {
        "struct Foo { a, b } Foo { a: 12 }",
        span!(20, 33), LitObjectMissingField { field, .. } => {
            assert_eq!(field.as_ref(), "b");
        }
    };

    assert_errors! {
        "struct Foo { a, b } Foo { not_field: 12 }",
        span!(26, 35), LitObjectNotField { field, .. } => {
            assert_eq!(field.as_ref(), "not_field");
        }
    };

    assert_errors! {
        "None(1)",
        span!(4, 7), BadArgumentCount { expected: 0, actual: 1, .. }
    };
}
