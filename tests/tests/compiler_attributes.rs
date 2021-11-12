use rune::compiling::CompileErrorKind::*;
use rune::Span;
use rune_tests::*;

#[test]
fn test_bad_attributes() {
    assert_compile_error! {
        r#"pub fn main() { #[foo] #[bar] let x = 1; }"#,
        span, Custom { message } => {
            assert_eq!(message, "attributes are not supported");
            assert_eq!(span, Span::new(16, 29));
        }
    };
}
