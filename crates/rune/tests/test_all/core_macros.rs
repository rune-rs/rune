#[test]
fn test_asserts() {
    rune!(() => fn main() { assert!(true) });
    rune!(() => fn main() { assert_eq!(1 + 1, 2) });
}

#[test]
fn test_stringify() {
    let out: String = rune!(String => fn main() { stringify!(assert_eq!(1 + 1, 2)) });
    assert_eq!("assert_eq ! ( 1 + 1 , 2 )", out);
}
