prelude!();

use ErrorKind::*;

#[test]
fn test_number_literals() {
    assert_parse!(r#"pub fn main() { -9223372036854775808 }"#);
    assert_parse!(
        r#"pub fn main() { -0b1000000000000000000000000000000000000000000000000000000000000000 }"#
    );
    assert_parse!(
        r#"pub fn main() { 0b0111111111111111111111111111111111111111111111111111111111111111 }"#
    );

    assert_errors! {
        r#"pub fn main() { -0aardvark }"#,
        span!(17, 26), BadNumberLiteral { .. }
    };

    assert_errors! {
        r#"pub fn main() { -9223372036854775809 }"#,
        span!(16, 36), BadNumberOutOfBounds { .. }
    };

    assert_parse!(r#"pub fn main() { 9223372036854775807 }"#);
    assert_errors! {
        r#"pub fn main() { 9223372036854775808 }"#,
        span!(16, 35), BadNumberOutOfBounds { .. }
    };

    assert_errors! {
        r#"pub fn main() { 0b1000000000000000000000000000000000000000000000000000000000000000 }"#,
        span!(16, 82), BadNumberOutOfBounds { .. }
    };
}
