use rune_tests::*;

#[test]
fn test_literals() {
    assert_eq!(
        rune!(String => pub fn main() { "Hello World" }),
        "Hello World"
    );

    assert_eq!(
        rune!(runestick::Bytes => pub fn main() { b"Hello World" }),
        b"Hello World"[..]
    );

    assert_eq!(rune!(i64 => pub fn main() { 0xff }), 0xff);
    assert_eq!(rune!(i64 => pub fn main() { -0xff }), -0xff);
    assert_eq!(rune!(i64 => pub fn main() { -42 }), -42);
    assert_eq!(rune!(i64 => pub fn main() { 0b10010001 }), 0b10010001);
    assert_eq!(rune!(i64 => pub fn main() { -0b10010001 }), -0b10010001);
    assert_eq!(rune!(i64 => pub fn main() { 0o77 }), 0o77);
    assert_eq!(rune!(i64 => pub fn main() { -0o77 }), -0o77);

    assert_eq!(rune!(u8 => pub fn main() { b'0' }), b'0');
    assert_eq!(rune!(u8 => pub fn main() { b'\xaf' }), b'\xaf');

    assert_eq!(rune!(char => pub fn main() { '\x60' }), '\x60');
    assert_eq!(rune!(char => pub fn main() { '\u{1F4AF}' }), '\u{1F4AF}');
    assert_eq!(rune!(char => pub fn main() { '💯' }), '💯');

    assert_eq!(rune!(f64 => pub fn main() { 42.42 }), 42.42);
    assert_eq!(rune!(f64 => pub fn main() { -42.42 }), -42.42);
    assert_eq!(rune!(f64 => pub fn main() { 1.9e10 }), 1.9e10);
    assert_eq!(rune!(f64 => pub fn main() { 1e10 }), 1e10);
}

#[test]
fn test_string_literals() {
    assert_eq!(
        rune!(String => pub fn main() { "
    " }),
        "\n    "
    );

    assert_eq!(
        rune!(String => pub fn main() { "\
    " }),
        ""
    );

    assert_eq!(
        rune!(String => pub fn main() { "\
    a \
\
    b" }),
        "a b"
    );
}

#[test]
fn test_byte_string_literals() {
    assert_eq!(
        rune!(Bytes => pub fn main() { b"
    " }),
        b"\n    "[..]
    );

    assert_eq!(
        rune!(Bytes => pub fn main() { b"\
    " }),
        b""[..]
    );

    assert_eq!(
        rune!(Bytes => pub fn main() { b"\
    a \
\
    b" }),
        b"a b"[..]
    );
}
