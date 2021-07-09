use rune_tests::*;

macro_rules! test_case {
    ($($tt:tt)*) => {
        let out: String = rune!(String => pub fn main() { format!($($tt)*) });
        assert_eq!(format!($($tt)*), out);
    }
}

#[test]
fn test_asserts() {
    rune!(() => pub fn main() { assert!(true) });
    rune!(() => pub fn main() { assert_eq!(1 + 1, 2) });
}

#[test]
fn test_stringify() {
    let out: String = rune!(String => pub fn main() { stringify!(assert_eq!(1 + 1, 2)) });
    assert_eq!("assert_eq ! ( 1 + 1 , 2 )", out);
}

#[test]
fn test_unicode() {
    test_case!("{name:😊^10}", name = "😞😞😞😞😞");
    test_case!("{name:﷽^10}", name = "𒈙");
    test_case!("{}", '㒨');
    test_case!("Hello {var}", var = '㒨');
}

#[test]
fn test_format() {
    test_case!("Hello, World");
    test_case!("Hello, {name}", name = "John Doe");
    test_case!("Hello, {1} {0}", "John", "Doe");
    test_case!("Hello, {} {0} {}", "John", "Doe");

    let out: String =
        rune!(String => pub fn main() { format!("Hello, {}" + " {0} {}", "John", "Doe") });
    assert_eq!(format!("Hello, {} {0} {}", "John", "Doe"), out);
}

#[test]
fn test_strings() {
    test_case!("{}", "test\tstring");
    test_case!("{:?}", "test\tstring");

    test_case!("{:>99}", "test\tstring");
    test_case!("{:>99?}", "test\tstring");
    test_case!("{:^99}", "test\tstring");
    test_case!("{:^99?}", "test\tstring");
    test_case!("{:>99}", "test\tstring");
    test_case!("{:>99?}", "test\tstring");

    // NB: sign aware zero expansion is ignored for strings.
    test_case!("{:>099}", "test\tstring");
    test_case!("{:>099?}", "test\tstring");
    test_case!("{:^099}", "test\tstring");
    test_case!("{:^099?}", "test\tstring");
    test_case!("{:>099}", "test\tstring");
    test_case!("{:>099?}", "test\tstring");

    test_case!("{:/>99}", "test\tstring");
    test_case!("{:/>99?}", "test\tstring");
    test_case!("{:/^99}", "test\tstring");
    test_case!("{:/^99?}", "test\tstring");
    test_case!("{:/>99}", "test\tstring");
    test_case!("{:/>99?}", "test\tstring");

    test_case!("{:\n>99}", "test\tstring");
    test_case!("{:\n>99?}", "test\tstring");
    test_case!("{:\n^99}", "test\tstring");
    test_case!("{:\n^99?}", "test\tstring");
    test_case!("{:\n>99}", "test\tstring");
    test_case!("{:\n>99?}", "test\tstring");
}

#[test]
fn test_float_formatting() {
    test_case!("{:.10}", 3.141592);
    test_case!("{:.*}", 10, 3.141592);
}

#[test]
fn test_number_formatting() {
    test_case!("{:<013}", -42);
    test_case!("{:^013}", -42);
    test_case!("{:>013}", -42);

    test_case!("{:<013}", 42);
    test_case!("{:^013}", 42);
    test_case!("{:>013}", 42);

    test_case!("{:/<13}", 42);
    test_case!("{:/^13}", 42);
    test_case!("{:/>13}", 42);

    test_case!("{:/<13x}", 42);
    test_case!("{:/^13x}", 42);
    test_case!("{:/>13x}", 42);

    test_case!("{:/<13X}", 42);
    test_case!("{:/^13X}", 42);
    test_case!("{:/>13X}", 42);

    test_case!("{:/<13b}", 42);
    test_case!("{:/^13b}", 42);
    test_case!("{:/>13b}", 42);
}
