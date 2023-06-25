prelude!();

#[test]
fn test_int_conversion() {
    let result: char = rune! {
        use std::char;
        pub fn main() {
            let a = 'A';
            let ai = char::to_i64(a);
            char::from_i64(ai).unwrap()
        }
    };

    assert_eq!(result, 'A');

    let result: char = rune! {
        pub fn main() {
            let ai = 0x41;
            char::from_i64(ai).unwrap()
        }
    };

    assert_eq!(result, 'A');
}
