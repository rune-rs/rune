#![allow(clippy::unusual_byte_groupings)]
#![allow(clippy::inconsistent_digit_grouping)]

prelude!();

#[test]
fn number_literals_oob() {
    use ErrorKind::*;

    assert_parse!("-9223372036854775808");
    assert_parse!("-0b1000000000000000000000000000000000000000000000000000000000000000");
    assert_parse!("0b0111111111111111111111111111111111111111111111111111111111111111");

    assert_errors! {
        "-0aardvark",
        span!(1, 10), BadNumberLiteral
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

#[test]
fn test_literals() {
    let out: String = rune!("Hello World");
    assert_eq!(out, "Hello World");
    let out: Bytes = rune!(b"Hello World");
    assert_eq!(out, b"Hello World"[..]);
    let out: u8 = rune!(b'0');
    assert_eq!(out, b'0');
    let out: u8 = rune!(b'\xaf');
    assert_eq!(out, b'\xaf');
    let out: char = rune!('\x60');
    assert_eq!(out, '\x60');
    let out: char = rune!('\u{1F4AF}');
    assert_eq!(out, '\u{1F4AF}');
    let out: char = rune!('💯');
    assert_eq!(out, '💯');
}

#[test]
fn test_string_literals() {
    let out: String = rune!(
        "
    "
    );
    assert_eq!(out, "\n    ");

    let out: String = rune!(
        "\
    "
    );
    assert_eq!(out, "");

    let out: String = rune!(
        "\
    a \
\
    b"
    );
    assert_eq!(out, "a b");
}

#[test]
fn test_byte_string_literals() {
    let out: Bytes = rune!(
        b"
    "
    );
    assert_eq!(out, b"\n    "[..]);

    let out: Bytes = rune!(
        b"\
    "
    );
    assert_eq!(out, b""[..]);

    let out: Bytes = rune!(
        b"\
    a \
\
    b"
    );
    assert_eq!(out, b"a b"[..]);
}

#[test]
fn test_number_literals() {
    macro_rules! test_case {
        ($lit:expr) => {
            test_case!($lit, i64);
        };

        ($lit:expr, $ty:ty) => {
            let out: $ty = rune!($lit);
            assert_eq!(out, $lit);
        };
    }

    test_case!(0xff);
    test_case!(-0xff);

    test_case!(0xf_f);
    test_case!(-0xf_f);

    test_case!(42);
    test_case!(-42);

    test_case!(4_2);
    test_case!(-4_2);

    test_case!(0b1001_0001);
    test_case!(-0b1001_0001);

    test_case!(0b10010001);
    test_case!(-0b10010001);

    test_case!(0o77);
    test_case!(0o7_7);

    test_case!(-0o77);
    test_case!(-0o7_7);

    test_case!(42.42, f32);
    test_case!(-42.42, f32);

    test_case!(42_.42, f32);
    test_case!(4_2.42, f32);
    test_case!(42.4_2, f32);
    test_case!(4_2.4_2, f32);

    test_case!(1.9e10, f64);
    test_case!(-1.9e10, f64);

    test_case!(1_.9e10, f64);
    test_case!(1.9e1_0, f64);

    test_case!(1e10, f64);
}
