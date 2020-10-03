use crate::testing::*;

#[test]
fn test_literals() {
    assert_eq!(rune!(String => fn main() { "Hello World" }), "Hello World");
    assert_eq!(
        rune!(runestick::Bytes => fn main() { b"Hello World" }),
        b"Hello World"[..]
    );

    assert_eq!(rune!(i64 => fn main() { 0xff }), 0xff);
    assert_eq!(rune!(i64 => fn main() { -0xff }), -0xff);

    assert_eq!(rune!(i64 => fn main() { 0b10010001 }), 0b10010001);
    assert_eq!(rune!(i64 => fn main() { -0b10010001 }), -0b10010001);

    assert_eq!(rune!(i64 => fn main() { 0o77 }), 0o77);
    assert_eq!(rune!(i64 => fn main() { -0o77 }), -0o77);

    assert_eq!(rune!(u8 => fn main() { b'0' }), b'0');
    assert_eq!(rune!(u8 => fn main() { b'\xaf' }), b'\xaf');

    assert_eq!(rune!(char => fn main() { '\x60' }), '\x60');
    assert_eq!(rune!(char => fn main() { '\u{1F4AF}' }), '\u{1F4AF}');
    assert_eq!(rune!(char => fn main() { '💯' }), '💯');
}

#[test]
fn test_string_literals() {
    assert_eq!(
        rune!(String => fn main() { "
    " }),
        "\n    "
    );

    assert_eq!(
        rune!(String => fn main() { "\
    " }),
        ""
    );

    assert_eq!(
        rune!(String => fn main() { "\
    a \
    
    b" }),
        "a b"
    );
}

#[test]
fn test_byte_string_literals() {
    assert_eq!(
        rune!(Bytes => fn main() { b"
    " }),
        b"\n    "[..]
    );

    assert_eq!(
        rune!(Bytes => fn main() { b"\
    " }),
        b""[..]
    );

    assert_eq!(
        rune!(Bytes => fn main() { b"\
    a \
    
    b" }),
        b"a b"[..]
    );
}
