use hegel::generators::{self, Generator};
use rune::Context;
use rune_hegel_tests::try_compile;

/// Property: the lexer, parser and compiler never panic on arbitrary unicode
/// input, in either script or module mode.
#[test]
fn compile_never_panics_on_arbitrary_text() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let source: String = tc.draw(hegel::one_of!(
            generators::text().boxed(),
            // Strings built from raw codepoints, hitting control characters and
            // supplementary planes more often than `text()` does.
            generators::vecs(
                generators::integers::<u32>()
                    .max_value(0x10_FFFF)
                    .map(|n| char::from_u32(n).unwrap_or('\u{FFFD}'))
            )
            .map(|v| v.into_iter().collect())
            .boxed(),
        ));
        let script = tc.draw(generators::booleans());

        try_compile(&context, &source, script);
    })
    .run();
}

fn token_vocabulary() -> Vec<&'static str> {
    vec![
        "fn", "let", "if", "else", "while", "for", "in", "loop", "match", "enum", "struct", "impl",
        "pub", "use", "mod", "const", "break", "continue", "return", "async", "await", "yield",
        "select", "true", "false", "self", "is", "not", "crate", "super", "default", "{", "}", "(",
        ")", "[", "]", ",", ";", ":", "::", ".", "..", "..=", "=>", "->", "#", "#{", "=", "==",
        "!=", "<", ">", "<=", ">=", "+", "-", "*", "/", "%", "&&", "||", "!", "&", "|", "^", "<<",
        ">>", "+=", "-=", "*=", "/=", "?", "@", "$", "'label", "_", "x", "foo", "Bar", "テスト",
        "0", "1", "42", "9223372036854775807", "0xff", "0o77", "0b1010", "1.5", "1e300",
        "\"string\"", "'c'", "`template ${x}`", "b\"bytes\"", "b'b'", "#[test]", "//! doc",
        "// comment", "/* block */",
    ]
}

/// Property: the parser and compiler never panic on "script-shaped" input —
/// random sequences of valid Rune tokens.
#[test]
fn compile_never_panics_on_token_soup() {
    let context = Context::with_default_modules().expect("failed to build context");
    let vocabulary = token_vocabulary();

    hegel::Hegel::new(|tc| {
        let count = tc.draw(generators::integers::<usize>().max_value(60));
        let mut source = String::new();

        for _ in 0..count {
            let token = tc.draw(generators::sampled_from(vocabulary.clone()));
            source.push_str(token);
            let newline = tc.draw(generators::booleans());
            source.push(if newline { '\n' } else { ' ' });
        }

        let script = tc.draw(generators::booleans());
        try_compile(&context, &source, script);
    })
    .run();
}

fn nested_source(shape: &str, depth: usize) -> String {
    let (open, close) = match shape {
        "paren" => ("(", ")"),
        "array" => ("[", "]"),
        "block" => ("{", "}"),
        "neg" => ("-(", ")"),
        "object" => ("#{a: ", "}"),
        "call" => ("f(", ")"),
        _ => unreachable!(),
    };

    let mut source = String::with_capacity(depth * (open.len() + close.len()) + 16);

    if shape == "call" {
        source.push_str("fn f(v) { v } ");
    }

    for _ in 0..depth {
        source.push_str(open);
    }
    source.push('0');
    for _ in 0..depth {
        source.push_str(close);
    }

    source
}

/// Property: moderately deep expression nesting compiles (or errors) without
/// crashing the process. Depth is capped at 20: deeper nesting aborts the
/// process with a stack overflow in the recursive parser (rune-rs/rune#1040).
#[test]
fn nested_expressions_compile_without_crashing() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let depth = tc.draw(generators::integers::<usize>().min_value(1).max_value(20));
        let shape = tc.draw(generators::sampled_from(vec![
            "paren", "array", "block", "neg", "object", "call",
        ]));

        let source = nested_source(shape, depth);
        try_compile(&context, &source, true);
    })
    .run();
}
