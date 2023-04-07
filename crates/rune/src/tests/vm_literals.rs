#![allow(clippy::unusual_byte_groupings)]

prelude!();

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

#[test]
fn test_number_literals() {
    macro_rules! test_case {
        ($lit:expr) => {
            test_case!($lit, i64);
        };

        ($lit:expr, $ty:ty) => {
            let out: $ty = rune!(
                pub fn main() {
                    $lit
                }
            );
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

    // TODO: we need a different float parsing routine to support _ in floats.
    // test_case!(42_.42, f32);
    // test_case!(4_2.42, f32);
    // test_case!(42.4_2, f32);
    // test_case!(4_2.4_2, f32);

    test_case!(1.9e10, f64);
    test_case!(-1.9e10, f64);

    // TODO: we need a different float parsing routine to support _ in floats.
    // test_case!(1_.9e10, f64);
    // test_case!(1.9e1_0, f64);

    test_case!(1e10, f64);
}
