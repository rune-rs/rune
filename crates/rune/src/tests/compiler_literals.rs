prelude!();

use CompileErrorKind::{ParseError, ResolveError};
use ParseErrorKind::*;
use ResolveErrorKind::*;

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
        span, ResolveError(BadNumberLiteral { .. }) => {
            assert_eq!(span, span!(17, 26));
        }
    };

    assert_compile_error! {
        r#"pub fn main() { -9223372036854775809 }"#,
        span, ParseError(BadNumberOutOfBounds { .. }) => {
            assert_eq!(span, span!(16, 36));
        }
    };

    assert_parse!(r#"pub fn main() { 9223372036854775807 }"#);
    assert_compile_error! {
        r#"pub fn main() { 9223372036854775808 }"#,
        span, ParseError(BadNumberOutOfBounds { .. }) => {
            assert_eq!(span, span!(16, 35));
        }
    };

    assert_compile_error! {
        r#"pub fn main() { 0b1000000000000000000000000000000000000000000000000000000000000000 }"#,
        span, ParseError(BadNumberOutOfBounds { .. }) => {
            assert_eq!(span, span!(16, 82));
        }
    };
}
