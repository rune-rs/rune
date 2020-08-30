use rune_testing::*;

#[test]
fn test_number_literals() {
    test_parse!(r#"fn main() { -9223372036854775808 }"#);
    test_parse!(
        r#"fn main() { -0b1000000000000000000000000000000000000000000000000000000000000000 }"#
    );
    test_parse!(
        r#"fn main() { 0b0111111111111111111111111111111111111111111111111111111111111111 }"#
    );

    test_compile_error! {
        r#"fn main() { -0aardvark }"#,
        ParseError { error: BadNumberLiteral { span, .. }} => {
            assert_eq!(span, Span::new(12, 22));
        }
    };

    test_compile_error! {
        r#"fn main() { -9223372036854775809 }"#,
        ParseError { error: BadNumberOutOfBounds { span, .. }} => {
            assert_eq!(span, Span::new(12, 32));
        }
    };

    test_parse!(r#"fn main() { 9223372036854775807 }"#);
    test_compile_error! {
        r#"fn main() { 9223372036854775808 }"#,
        ParseError { error: BadNumberOutOfBounds { span, .. }} => {
            assert_eq!(span, Span::new(12, 31));
        }
    };

    test_compile_error! {
        r#"fn main() { 0b1000000000000000000000000000000000000000000000000000000000000000 }"#,
        ParseError { error: BadNumberOutOfBounds { span, .. }} => {
            assert_eq!(span, Span::new(12, 78));
        }
    };
}
