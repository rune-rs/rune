use rune_tests::*;

#[test]
fn test_variant_typing() {
    assert! {
        rune!(bool => pub fn main() { Err(0) is Result }),
    };

    assert! {
        rune!(bool => pub fn main() { Ok(0) is Result }),
    };

    assert! {
        rune!(bool => pub fn main() { Some(0) is Option }),
    };

    assert! {
        rune!(bool => pub fn main() { None is Option }),
    };

    assert! {
        rune! { bool =>
            enum Custom { A, B(a) }
            pub fn main() { Custom::A is Custom }
        },
    };

    assert! {
        rune! { bool =>
            enum Custom { A, B(a) }
            pub fn main() { Custom::B(42) is Custom }
        },
    };

    assert! {
        !rune! { bool =>
            enum Custom { A, B(a) }
            pub fn main() { Custom::A is Option }
        },
    };

    assert! {
        rune! { bool =>
            enum Custom { A, B(a) }
            pub fn main() { Custom::A is not Option }
        },
    };
}
