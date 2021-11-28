use rune::runtime::Bytes;
use rune_tests::*;

#[test]
fn test_literals() {
    let out: String = rune!(
        pub fn main() {
            "Hello World"
        }
    );
    assert_eq!(out, "Hello World");

    let out: Bytes = rune!(
        pub fn main() {
            b"Hello World"
        }
    );
    assert_eq!(out, b"Hello World"[..]);

    let out: i64 = rune!(
        pub fn main() {
            0xff
        }
    );
    assert_eq!(out, 0xff);
    let out: i64 = rune!(
        pub fn main() {
            -0xff
        }
    );
    assert_eq!(out, -0xff);
    let out: i64 = rune!(
        pub fn main() {
            -42
        }
    );
    assert_eq!(out, -42);
    let out: i64 = rune!(
        pub fn main() {
            0b10010001
        }
    );
    assert_eq!(out, 0b10010001);
    let out: i64 = rune!(
        pub fn main() {
            -0b10010001
        }
    );
    assert_eq!(out, -0b10010001);
    let out: i64 = rune!(
        pub fn main() {
            0o77
        }
    );
    assert_eq!(out, 0o77);
    let out: i64 = rune!(
        pub fn main() {
            -0o77
        }
    );
    assert_eq!(out, -0o77);

    let out: u8 = rune!(
        pub fn main() {
            b'0'
        }
    );
    assert_eq!(out, b'0');
    let out: u8 = rune!(
        pub fn main() {
            b'\xaf'
        }
    );
    assert_eq!(out, b'\xaf');

    let out: char = rune!(
        pub fn main() {
            '\x60'
        }
    );
    assert_eq!(out, '\x60');
    let out: char = rune!(
        pub fn main() {
            '\u{1F4AF}'
        }
    );
    assert_eq!(out, '\u{1F4AF}');
    let out: char = rune!(
        pub fn main() {
            '💯'
        }
    );
    assert_eq!(out, '💯');

    let out: f64 = rune!(
        pub fn main() {
            42.42
        }
    );
    assert_eq!(out, 42.42);
    let out: f64 = rune!(
        pub fn main() {
            -42.42
        }
    );
    assert_eq!(out, -42.42);
    let out: f64 = rune!(
        pub fn main() {
            1.9e10
        }
    );
    assert_eq!(out, 1.9e10);
    let out: f64 = rune!(
        pub fn main() {
            1e10
        }
    );
    assert_eq!(out, 1e10);
}

#[test]
fn test_string_literals() {
    let out: String = rune!(
        pub fn main() {
            "
    "
        }
    );
    assert_eq!(out, "\n    ");

    let out: String = rune!(
        pub fn main() {
            "\
    "
        }
    );
    assert_eq!(out, "");

    let out: String = rune!(
        pub fn main() {
            "\
    a \
\
    b"
        }
    );
    assert_eq!(out, "a b");
}

#[test]
fn test_byte_string_literals() {
    let out: Bytes = rune!(
        pub fn main() {
            b"
    "
        }
    );
    assert_eq!(out, b"\n    "[..]);

    let out: Bytes = rune!(
        pub fn main() {
            b"\
    "
        }
    );
    assert_eq!(out, b""[..]);

    let out: Bytes = rune!(
        pub fn main() {
            b"\
    a \
\
    b"
        }
    );
    assert_eq!(out, b"a b"[..]);
}
