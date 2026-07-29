prelude!();

macro_rules! test_case {
    ($($tt:tt)*) => {
        let out: String = rune!(format!($($tt)*));
        assert_eq!(format!($($tt)*), out);
    }
}

#[test]
fn test_asserts() {
    let _: () = rune!(
        assert!(true);
    );

    let _: () = rune!(
        assert_eq!(1 + 1, 2);
    );
}

#[test]
fn test_stringify() {
    let out: String = rune!(stringify!(assert_eq!(1 + 1, 2)));
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

    let out: String = rune!(format!("Hello, {}" + " {0} {}", "John", "Doe"));
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
    test_case!("{:.10}", 3.1415);
    test_case!("{:.*}", 10, 3.1415);
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

/// A macro is invoked through any of the three delimiters, as in Rust. Its
/// input is a raw token stream either way, so which one is used says nothing
/// about how it is expanded.
#[test]
fn macro_call_delimiters() {
    let parens: String = eval(r#"format!("{}", 42)"#);
    let brackets: String = eval(r#"format!["{}", 42]"#);
    let braces: String = eval(r#"format!{"{}", 42}"#);

    assert_eq!(parens, "42");
    assert_eq!(brackets, "42");
    assert_eq!(braces, "42");
}

/// A macro call is the head of the chain applied to it, whichever delimiter it
/// was invoked through.
#[test]
fn macro_call_delimiters_chained() {
    let parens: usize = eval(r#"format!("{}", 42).len()"#);
    let brackets: usize = eval(r#"format!["{}", 42].len()"#);

    assert_eq!(parens, 2);
    assert_eq!(brackets, 2);
}
