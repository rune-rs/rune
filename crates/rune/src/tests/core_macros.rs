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

/// The macros the standard library provides split their input into arguments
/// with the compiler's own parser, so an argument is the whole expression it
/// was written as.
///
/// The cases below are what splitting on commas by hand gets wrong: the commas
/// between the parameters of a closure and between the parameters of a
/// turbofish are not inside a delimiter, and a block is one argument however
/// many statements it is made of.
#[test]
fn an_argument_is_the_expression_it_was_written_as() {
    let closure: String = eval(r#"format!("{}", (|a, b| a + b)(1, 2))"#);
    assert_eq!(closure, "3");

    let closure: String = eval(r#"format!("{}", [|a, b| a + b][0](1, 2))"#);
    assert_eq!(closure, "3");

    let block: String = eval(r#"format!("{}", { let a = 1; a + 1 })"#);
    assert_eq!(block, "2");

    // Nothing in the standard library takes generic parameters, so the only
    // thing a turbofish can do here is fail to resolve - but it fails as the
    // one path it was written as rather than as two arguments.
    assert_errors! {
        r#"pub fn main() { format!("{}", Vec::<i64, i64>::new()); }"#,
        span, ErrorKind::MissingItemParameters { parameters, .. } => {
            // Both parameters belong to the one path, which is what says the
            // comma between them did not end the argument.
            assert_eq!(parameters.len(), 2);
            assert_eq!(span.range(), 30..50);
        }
    };
}

/// A block is parsed into a node which has a body of its own, and the body of
/// an empty one has nothing in it.
///
/// The tokens of an argument are pulled back out of the tree it was split with,
/// and a node with no children looks exactly like a token to anything which
/// only asks whether it is a leaf - so an empty body used to be handed on as if
/// it were a token, which nothing downstream could make sense of.
#[test]
fn an_empty_block_is_an_argument() {
    let empty: String = eval(r#"format!("{:?}", {})"#);
    assert_eq!(empty, "()");

    let _: () = rune!(
        assert!({} is Tuple);
    );

    let nested: String = eval(r#"format!("{:?}", { {} })"#);
    assert_eq!(nested, "()");
}

/// An argument which is not separated from the next one, and a separator with
/// no argument between it and the next, are both reported where they are
/// written rather than being read as something else.
#[test]
fn arguments_are_separated() {
    assert_errors! {
        r#"pub fn main() { println!("{}" 1); }"#,
        span, ErrorKind::Custom { error } => {
            assert_eq!(error.as_str(), "expected `,`");
            assert_eq!(span.range(), 30..31);
        }
    };

    assert_errors! {
        r#"pub fn main() { println!("{}",, 1); }"#,
        span, ErrorKind::Custom { error } => {
            assert_eq!(error.as_str(), "expected an expression");
            assert_eq!(span.range(), 30..31);
        }
    };
}
