prelude!();

#[test]
fn test_int_fns() {
    let n: i64 = rune! {
        pub fn main() {
            1.min(2)
        }
    };
    assert_eq!(n, 1);

    let n: i64 = rune! {
        pub fn main() {
            std::i64::min(1, 2)
        }
    };
    assert_eq!(n, 1);

    let n: i64 = rune! {
        pub fn main() {
            1.max(2)
        }
    };
    assert_eq!(n, 2);

    let n: i64 = rune! {
        pub fn main() {
            std::i64::max(1, 2)
        }
    };
    assert_eq!(n, 2);

    let n: i64 = rune! {
        pub fn main() {
            (-10).abs()
        }
    };
    assert_eq!(n, 10);

    let n: i64 = rune! {
        pub fn main() {
            std::i64::abs(-10)
        }
    };
    assert_eq!(n, 10);

    let n: i64 = rune! {
        pub fn main() {
            (12).pow(3)
        }
    };
    assert_eq!(n, 1728);

    let n: i64 = rune! {
        pub fn main() {
            std::i64::pow(12, 3)
        }
    };
    assert_eq!(n, 1728);
}
