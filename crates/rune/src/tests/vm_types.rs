prelude!();

#[test]
fn test_variant_typing() {
    let out: bool = rune!(pub fn main() { Err(0) is Result });
    assert_eq!(out, true);

    let out: bool = rune!(pub fn main() { Ok(0) is Result });
    assert_eq!(out, true);

    let out: bool = rune!(pub fn main() { Some(0) is Option });
    assert_eq!(out, true);

    let out: bool = rune!(pub fn main() { None is Option });
    assert_eq!(out, true);

    let out: bool = rune! {
        enum Custom { A, B(a) }
        pub fn main() { Custom::A is Custom }
    };
    assert_eq!(out, true);

    let out: bool = rune! {
        enum Custom { A, B(a) }
        pub fn main() { Custom::B(42) is Custom }
    };
    assert_eq!(out, true);

    let out: bool = rune! {
        enum Custom { A, B(a) }
        pub fn main() { Custom::A is Option }
    };
    assert_eq!(out, false);

    let out: bool = rune! {
        enum Custom { A, B(a) }
        pub fn main() { Custom::A is not Option }
    };
    assert_eq!(out, true);
}
