prelude!();

use ErrorKind::*;

#[test]
fn test_use_variant_as_type() {
    assert_errors! {
        "Err(0) is Err",
        span!(10, 13), ExpectedMeta { meta, .. } => {
            assert_eq!(meta.to_string(), "variant ::std::result::Result::Err");
        }
    };
}

#[test]
fn break_outside_of_loop() {
    assert_errors! {
        "break;",
        span!(0, 5), BreakUnsupported
    };
}

#[test]
fn for_break_with_value() {
    assert_errors! {
        "for _ in 0..10 { break 42; }",
        span!(17, 25), BreakUnsupportedValue
    };
}

#[test]
fn continue_outside_of_loop() {
    assert_errors! {
        "continue;",
        span!(0, 8), ContinueUnsupported
    };
}

#[test]
fn test_pointers() {
    assert_errors! {
        "let n = 0; foo(&n); fn foo(n) {}",
        span!(15, 16), UnsupportedRef
    };
}

#[test]
fn test_template_strings() {
    assert_parse!(r"`hello \``");
    assert_parse!(r"`hello \$`");
}

#[test]
fn test_wrong_arguments() {
    assert_errors! {
        "Some(1, 2)",
        span!(0, 4), BadArgumentCount { expected: 1, actual: 2, .. }
    };

    assert_errors! {
        "None(1)",
        span!(0, 4), BadArgumentCount { expected: 0, actual: 1, .. }
    };
}

#[test]
fn test_bad_struct_declaration() {
    assert_errors! {
        "struct Foo { a, b } Foo { a: 12 }",
        span!(20, 23), LitObjectMissingField { field, .. } => {
            assert_eq!(field.as_ref(), "b");
        }
    };

    assert_errors! {
        "struct Foo { a, b } Foo { not_field: 12 }",
        span!(26, 35), LitObjectNotField { field, .. } => {
            assert_eq!(field.as_ref(), "not_field");
        }
    };

    assert_errors! {
        "None(1)",
        span!(0, 4), BadArgumentCount { expected: 0, actual: 1, .. }
    };
}

/// `move` and `const` start an expression the way `async` does.
///
/// Where the grammar decides whether another expression follows - the arguments
/// of a call, the elements of a vector, the operand of `return`, `yield` or
/// `break` - it did not count either of them, so a `move` closure could not be
/// written as an argument at all while `async move` could. That is the shape
/// every `move` closure is written in, and the one the book shows.
#[test]
fn move_and_const_start_an_expression() {
    let value: i64 = eval("fn work(op) { op(1, 2) } let n = 1; work(move |a, b| n + a + b)");
    assert_eq!(value, 4);

    let value: i64 = eval("let v = [move || 1, move || 2]; v[0]() + v[1]()");
    assert_eq!(value, 3);

    let value: i64 = eval("fn f() { return move || 7; } f()()");
    assert_eq!(value, 7);

    let value: i64 = eval("fn g() { yield move || 1; } let it = g(); it.next().unwrap()()");
    assert_eq!(value, 1);

    let value: i64 = eval("let f = loop { break move || 5; }; f()");
    assert_eq!(value, 5);

    let value: Vec<i64> = eval("let v = [1, 2]; v.iter().map(move |x| x * 2).collect::<Vec>()");
    assert_eq!(value, [2, 4]);

    // The same for a block which is evaluated while compiling.
    let value: i64 = eval("fn g(x) { x } g(const { 1 + 2 })");
    assert_eq!(value, 3);

    let value: Vec<i64> = eval("[const { 1 + 2 }]");
    assert_eq!(value, [3]);

    let value: i64 = eval("fn f() { return const { 4 }; } f()");
    assert_eq!(value, 4);

    // And what they mean is unchanged: what a `move` closure captures is moved,
    // so reading it afterwards is reported. Before this parsed at all, what was
    // reported instead was that the argument list did not close.
    let context = Context::with_default_modules().expect("Failed to build context");

    let mut sources = crate::tests::sources(
        "fn work(op) { op(1, 2) } \
         let n = 1; \
         work(move |a, b| n + a + b); \
         n",
    );

    let mut diagnostics = Diagnostics::new();

    let mut options = Options::default();
    options.script(true);

    let result = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_options(&options)
        .build();

    assert!(result.is_err(), "Reading what was moved should be reported");
}
