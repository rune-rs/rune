// The formatter is handed arbitrary text which does not have to compile, and
// what it owes in return is narrow: it may lay out something it did not
// understand however it likes, but it may not change what the source says, and
// running it on its own output has to be a no-op. A file which is formatted on
// every save is formatted over and over, so anything which is not a fixed point
// is a file which drifts.

use hegel::generators::{self, Generator};
use rune_hegel_tests::try_format;

fn token_vocabulary() -> Vec<&'static str> {
    vec![
        "fn", "let", "if", "else", "while", "for", "in", "loop", "match", "enum", "struct", "impl",
        "pub", "use", "mod", "const", "break", "continue", "return", "async", "await", "yield",
        "select", "true", "false", "self", "is", "not", "crate", "super", "default", "static", "{",
        "}", "(", ")", "[", "]", ",", ";", ":", "::", ".", "..", "..=", "=>", "->", "#", "#{", "=",
        "==", "!=", "<", ">", "<=", ">=", "+", "-", "*", "/", "%", "&&", "||", "!", "&", "|", "^",
        "<<", ">>", "+=", "-=", "*=", "/=", "?", "@", "$", "'label", "_", "x", "foo", "Bar",
        "テスト", "0", "1", "42", "9223372036854775807", "0xff", "0o77", "0b1010", "1.5", "1e300",
        "\"string\"", "'c'", "`template ${x}`", "`a{b`", "`a\\`b`", "b\"bytes\"", "b'b'",
        "#[test]", "//! doc", "// comment", "/* block */", "|a|", "||",
    ]
}

/// Property: the formatter never panics on arbitrary unicode input.
#[test]
fn formatting_never_panics_on_arbitrary_text() {
    hegel::Hegel::new(|tc| {
        let source: String = tc.draw(hegel::one_of!(
            generators::text().boxed(),
            generators::vecs(
                generators::integers::<u32>()
                    .max_value(0x10_FFFF)
                    .map(|n| char::from_u32(n).unwrap_or('\u{FFFD}'))
            )
            .map(|v| v.into_iter().collect())
            .boxed(),
        ));

        try_format(&source);
    })
    .settings(hegel::Settings::new().test_cases(2000))
    .run();
}

/// Property: formatting something which has already been formatted changes
/// nothing.
///
/// Anything else is a file which grows or churns a little more every time it is
/// saved, and a `--check` which fails on a file the formatter itself wrote.
#[test]
fn formatting_is_idempotent() {
    let vocabulary = token_vocabulary();

    hegel::Hegel::new(|tc| {
        let count = tc.draw(generators::integers::<usize>().max_value(40));
        let mut source = String::new();

        for _ in 0..count {
            let token = tc.draw(generators::sampled_from(vocabulary.clone()));
            source.push_str(token);
            let newline = tc.draw(generators::booleans());
            source.push(if newline { '\n' } else { ' ' });
        }

        let Some(once) = try_format(&source) else {
            return;
        };

        let Some(twice) = try_format(&once) else {
            panic!("Formatted output should format again:\n{once}");
        };

        assert_eq!(
            once, twice,
            "Formatting is not a fixed point.\nSource:\n{source}\nOnce:\n{once}\nTwice:\n{twice}"
        );
    })
    .settings(hegel::Settings::new().test_cases(5000))
    .run();
}
