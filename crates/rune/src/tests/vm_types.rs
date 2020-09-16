#[test]
fn test_variant_typing() {
    assert_eq! {
        rune! {
            bool => r#"fn main() { Err(0) is Result }"#
        },
        true,
    };

    assert_eq! {
        rune! {
            bool => r#"fn main() { Ok(0) is Result }"#
        },
        true,
    };

    assert_eq! {
        rune! {
            bool => r#"fn main() { Some(0) is Option }"#
        },
        true,
    };

    assert_eq! {
        rune! {
            bool => r#"fn main() { None is Option }"#
        },
        true,
    };

    assert_eq! {
        rune! {
            bool => r#"
            enum Custom { A, B(a) }
            fn main() { Custom::A is Custom }
            "#
        },
        true,
    };

    assert_eq! {
        rune! {
            bool => r#"
            enum Custom { A, B(a) }
            fn main() { Custom::B(42) is Custom }
            "#
        },
        true,
    };

    assert_eq! {
        rune! {
            bool => r#"
            enum Custom { A, B(a) }
            fn main() { Custom::A is Option }
            "#
        },
        false,
    };

    assert_eq! {
        rune! {
            bool => r#"
            enum Custom { A, B(a) }
            fn main() { Custom::A is not Option }
            "#
        },
        true,
    };
}
