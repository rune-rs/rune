#[test]
fn test_const_values() {
    assert_eq!(
        true,
        rune!(bool => r#"const VALUE = true; fn main() { VALUE }"#)
    );
    assert_eq!(
        "Hello World",
        rune!(String => r#"const VALUE = "Hello World"; fn main() { VALUE }"#)
    );
}
