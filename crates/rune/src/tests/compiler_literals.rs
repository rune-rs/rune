prelude!();

use ErrorKind::*;

#[test]
fn number_literals_oob() {
    assert_parse!("-9223372036854775808");
    assert_parse!("-0b1000000000000000000000000000000000000000000000000000000000000000");
    assert_parse!("0b0111111111111111111111111111111111111111111111111111111111111111");

    assert_errors! {
        "-0aardvark",
        span!(1, 10), BadNumberLiteral { .. }
    };

    assert_errors! {
        "-9223372036854775809",
        span!(0, 20), BadSignedOutOfBounds { .. }
    };

    assert_parse!("9223372036854775807");
    assert_errors! {
        "9223372036854775808",
        span!(0, 19), BadSignedOutOfBounds { .. }
    };

    assert_errors! {
        "0b1000000000000000000000000000000000000000000000000000000000000000",
        span!(0, 66), BadSignedOutOfBounds { .. }
    };
}
