prelude!();

use ErrorKind::*;

#[test]
fn test_use_variant_as_type() {
    assert_errors! {
        r#"pub fn main() { Err(0) is Err }"#,
        span!(26, 29), ExpectedMeta { meta, .. } => {
            assert_eq!(meta.to_string(), "variant ::std::result::Result::Err");
        }
    };
}

#[test]
fn break_outside_of_loop() {
    assert_errors! {
        r#"pub fn main() { break; }"#,
        span!(16, 21), BreakOutsideOfLoop
    };
}

#[test]
fn continue_outside_of_loop() {
    assert_errors! {
        r#"pub fn main() { continue; }"#,
        span!(16, 24), ContinueOutsideOfLoop
    };
}

#[test]
fn test_pointers() {
    assert_errors! {
        r#"pub fn main() { let n = 0; foo(&n); } fn foo(n) {}"#,
        span!(31, 33), UnsupportedRef
    };
}

#[test]
fn test_template_strings() {
    assert_parse!(r"pub fn main() { `hello \`` }");
    assert_parse!(r"pub fn main() { `hello \$` }");
}

#[test]
fn test_wrong_arguments() {
    assert_errors! {
        r#"pub fn main() { Some(1, 2) }"#,
        span!(20, 26), UnsupportedArgumentCount { expected: 1, actual: 2, .. }
    };

    assert_errors! {
        r#"pub fn main() { None(1) }"#,
        span!(20, 23), UnsupportedArgumentCount { expected: 0, actual: 1, .. }
    };
}

#[test]
fn test_bad_struct_declaration() {
    assert_errors! {
        r#"struct Foo { a, b } pub fn main() { Foo { a: 12 } }"#,
        span!(36, 49), LitObjectMissingField { field, .. } => {
            assert_eq!(field.as_ref(), "b");
        }
    };

    assert_errors! {
        r#"struct Foo { a, b } pub fn main() { Foo { not_field: 12 } }"#,
        span!(42, 51), LitObjectNotField { field, .. } => {
            assert_eq!(field.as_ref(), "not_field");
        }
    };

    assert_errors! {
        r#"pub fn main() { None(1) }"#,
        span!(20, 23), UnsupportedArgumentCount { expected: 0, actual: 1, .. }
    };
}
