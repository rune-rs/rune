use rune_tests::*;

#[test]
fn test_ignore_binding() {
    assert_eq! {
        rune! { bool =>
            fn returns_unit(n) {
                let _ = 100;
            }

            pub fn main() {
                returns_unit(1) is unit
            }
        },
        true,
    };
}

#[test]
fn test_name_binding() {
    assert_eq! {
        rune! { bool =>
            fn returns_unit(n) {
                let a = 100;
            }

            pub fn main() {
                returns_unit(1) is unit
            }
        },
        true,
    };
}

#[test]
fn test_match_binding() {
    assert_eq! {
        rune! { bool =>
            fn returns_unit(n) {
                let [..] = [1, 2, 3];
            }

            pub fn main() {
                returns_unit(1) is unit
            }
        },
        true,
    };
}
