#[test]
fn test_hex() {
    assert_eq! {
        rune!(i64 => r#"fn main() { 0xff }"#),
        255,
    };

    assert_eq! {
        rune!(i64 => r#"fn main() { -0xff }"#),
        -255,
    };
}

#[test]
fn test_binary() {
    assert_eq! {
        rune!(i64 => r#"fn main() { 0b10010001 }"#),
        145,
    };

    assert_eq! {
        rune!(i64 => r#"fn main() { -0b10010001 }"#),
        -145,
    };
}

#[test]
fn test_octal() {
    assert_eq! {
        rune!(i64 => r#"fn main() { 0o77 }"#),
        63,
    };

    assert_eq! {
        rune!(i64 => r#"fn main() { -0o77 }"#),
        -63,
    };
}
