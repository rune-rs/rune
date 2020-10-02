#[test]
fn test_variant_typing() {
    assert_eq! {
        rune!(bool => fn main() { Err(0) is Result }),
        true,
    };

    assert_eq! {
        rune!(bool => fn main() { Ok(0) is Result }),
        true,
    };

    assert_eq! {
        rune!(bool => fn main() { Some(0) is Option }),
        true,
    };

    assert_eq! {
        rune!(bool => fn main() { None is Option }),
        true,
    };

    assert_eq! {
        rune! { bool =>
            enum Custom { A, B(a) }
            fn main() { Custom::A is Custom }
        },
        true,
    };

    assert_eq! {
        rune! { bool =>
            enum Custom { A, B(a) }
            fn main() { Custom::B(42) is Custom }
        },
        true,
    };

    assert_eq! {
        rune! { bool =>
            enum Custom { A, B(a) }
            fn main() { Custom::A is Option }
        },
        false,
    };

    assert_eq! {
        rune! { bool =>
            enum Custom { A, B(a) }
            fn main() { Custom::A is not Option }
        },
        true,
    };
}
