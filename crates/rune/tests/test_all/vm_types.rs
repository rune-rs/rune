#[test]
fn test_variant_typing() {
    assert_eq! {
        rune!(bool => pub fn main() { Err(0) is Result }),
        true,
    };

    assert_eq! {
        rune!(bool => pub fn main() { Ok(0) is Result }),
        true,
    };

    assert_eq! {
        rune!(bool => pub fn main() { Some(0) is Option }),
        true,
    };

    assert_eq! {
        rune!(bool => pub fn main() { None is Option }),
        true,
    };

    assert_eq! {
        rune! { bool =>
            enum Custom { A, B(a) }
            pub fn main() { Custom::A is Custom }
        },
        true,
    };

    assert_eq! {
        rune! { bool =>
            enum Custom { A, B(a) }
            pub fn main() { Custom::B(42) is Custom }
        },
        true,
    };

    assert_eq! {
        rune! { bool =>
            enum Custom { A, B(a) }
            pub fn main() { Custom::A is Option }
        },
        false,
    };

    assert_eq! {
        rune! { bool =>
            enum Custom { A, B(a) }
            pub fn main() { Custom::A is not Option }
        },
        true,
    };
}
