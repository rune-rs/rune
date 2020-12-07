use rune_tests::*;

#[test]
fn test_number_literals() {
    assert_parse!(r#"pub fn main() { -9223372036854775808 }"#);
    assert_parse!(
        r#"pub fn main() { -0b1000000000000000000000000000000000000000000000000000000000000000 }"#
    );
    assert_parse!(
        r#"pub fn main() { 0b0111111111111111111111111111111111111111111111111111111111111111 }"#
    );

    assert_compile_error! {
        r#"pub fn main() { -0aardvark }"#,
        span, CompileErrorKind::ResolveError { error: BadNumberLiteral { .. } } => {
            assert_eq!(span, Span::new(17, 26));
        }
    };

    assert_compile_error! {
        r#"pub fn main() { -9223372036854775809 }"#,
        span, CompileErrorKind::ParseError { error: BadNumberOutOfBounds { .. }} => {
            assert_eq!(span, Span::new(16, 36));
        }
    };

    assert_parse!(r#"pub fn main() { 9223372036854775807 }"#);
    assert_compile_error! {
        r#"pub fn main() { 9223372036854775808 }"#,
        span, CompileErrorKind::ParseError { error: BadNumberOutOfBounds { .. }} => {
            assert_eq!(span, Span::new(16, 35));
        }
    };

    assert_compile_error! {
        r#"pub fn main() { 0b1000000000000000000000000000000000000000000000000000000000000000 }"#,
        span, CompileErrorKind::ParseError { error: BadNumberOutOfBounds { .. }} => {
            assert_eq!(span, Span::new(16, 82));
        }
    };
}
