use rune_testing::*;

#[test]
fn test_number_literals() {
    assert_parse!(r#"fn main() { -9223372036854775808 }"#);
    assert_parse!(
        r#"fn main() { -0b1000000000000000000000000000000000000000000000000000000000000000 }"#
    );
    assert_parse!(
        r#"fn main() { 0b0111111111111111111111111111111111111111111111111111111111111111 }"#
    );

    assert_compile_error! {
        r#"fn main() { -0aardvark }"#,
        span, ParseError { error: BadNumberLiteral { .. }} => {
            assert_eq!(span, Span::new(12, 22));
        }
    };

    assert_compile_error! {
        r#"fn main() { -9223372036854775809 }"#,
        span, ParseError { error: BadNumberOutOfBounds { .. }} => {
            assert_eq!(span, Span::new(12, 32));
        }
    };

    assert_parse!(r#"fn main() { 9223372036854775807 }"#);
    assert_compile_error! {
        r#"fn main() { 9223372036854775808 }"#,
        span, ParseError { error: BadNumberOutOfBounds { .. }} => {
            assert_eq!(span, Span::new(12, 31));
        }
    };

    assert_compile_error! {
        r#"fn main() { 0b1000000000000000000000000000000000000000000000000000000000000000 }"#,
        span, ParseError { error: BadNumberOutOfBounds { .. }} => {
            assert_eq!(span, Span::new(12, 78));
        }
    };
}
