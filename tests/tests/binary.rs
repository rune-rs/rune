use rune_tests::*;

#[test]
fn test_basic_operator_precedence() {
    let result = rune! { bool =>
        pub fn main() {
            10 < 5 + 10 && 5 > 4
        }
    };

    assert!(result);

    let result = rune! { bool =>
        pub fn main() {
            10 < 5 - 10 && 5 > 4
        }
    };

    assert!(!result);
}
