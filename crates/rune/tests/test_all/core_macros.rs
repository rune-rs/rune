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

#[test]
fn test_format() {
    let out: String = rune!(String => fn main() { format!("Hello, World") });
    assert_eq!("Hello, World", out);

    let out: String = rune!(String => fn main() { format!("Hello, {name}", name = "John Doe") });
    assert_eq!("Hello, John Doe", out);

    let out: String = rune!(String => fn main() { format!("Hello, {1} {0}", "John", "Doe") });
    assert_eq!("Hello, Doe John", out);

    let out: String = rune!(String => fn main() { format!("Hello, {} {0} {}", "John", "Doe") });
    assert_eq!("Hello, John John Doe", out);

    let out: String =
        rune!(String => fn main() { format!("Hello, {}" + " {0} {}", "John", "Doe") });
    assert_eq!("Hello, John John Doe", out);
}
