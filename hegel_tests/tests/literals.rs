use hegel::generators::{self, Generator};
use rune::Context;
use rune_hegel_tests::eval_result;

/// Property: rendering any `i64` as a decimal, hex, octal or binary literal
/// (optionally with `_` digit separators and an optional `i64` suffix) and
/// evaluating it yields the value back. Exercises the lexer's number handling
/// and the negative-literal special case in HIR lowering, including `i64::MIN`.
#[test]
fn integer_literal_roundtrip_all_bases() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let n: i64 = tc.draw(hegel::one_of!(
            generators::integers::<i64>(),
            generators::sampled_from(vec![i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX]),
        ));
        let base: u8 = tc.draw(generators::integers::<u8>().max_value(3));
        let separators = tc.draw(generators::booleans());
        let suffix = tc.draw(generators::booleans());

        let magnitude = n.unsigned_abs();

        let mut digits = match base {
            0 => format!("{magnitude}"),
            1 => format!("{magnitude:x}"),
            2 => format!("{magnitude:o}"),
            _ => format!("{magnitude:b}"),
        };

        if separators {
            let joined: Vec<String> = digits.chars().map(|c| c.to_string()).collect();
            digits = joined.join("_");
        }

        let prefix = match base {
            0 => "",
            1 => "0x",
            2 => "0o",
            _ => "0b",
        };

        let sign = if n < 0 { "-" } else { "" };
        let suffix = if suffix { "i64" } else { "" };
        let src = format!("{sign}{prefix}{digits}{suffix}");

        match eval_result::<i64>(&context, &src) {
            Ok(out) => assert_eq!(out, n, "source: {src}"),
            Err(error) => panic!("failed to evaluate literal {src}: {error:?}"),
        }
    })
    .run();
}

/// Property: rendering any finite `f64` with Rust's shortest round-trip
/// formatting and evaluating it as a Rune float literal yields the exact same
/// bits back.
#[test]
fn float_literal_roundtrip() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let f: f64 = tc.draw(
            generators::floats::<f64>()
                .allow_nan(false)
                .allow_infinity(false),
        );

        let src = format!("{f:?}");

        match eval_result::<f64>(&context, &src) {
            Ok(out) => assert_eq!(out.to_bits(), f.to_bits(), "expected {f:?}, got {out:?}\nsource: {src}"),
            Err(error) => panic!("failed to evaluate float literal {src}: {error:?}"),
        }
    })
    .run();
}

/// Escape a string so it can be embedded in a double-quoted Rune string
/// literal. When `force_unicode_escapes` is set every character is written as a
/// `\u{..}` escape to exercise the escape parser; otherwise only characters
/// that must be escaped are.
fn escape_rune_string(s: &str, force_unicode_escapes: bool) -> String {
    let mut out = String::new();

    for c in s.chars() {
        if force_unicode_escapes {
            out.push_str(&format!("\\u{{{:x}}}", c as u32));
            continue;
        }

        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }

    out
}

/// Property: any string, escaped and embedded as a Rune string literal,
/// evaluates back to the original string.
#[test]
fn string_literal_roundtrip() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let s: String = tc.draw(generators::text());
        let force_unicode_escapes = tc.draw(generators::booleans());

        let src = format!("\"{}\"", escape_rune_string(&s, force_unicode_escapes));

        match eval_result::<String>(&context, &src) {
            Ok(out) => assert_eq!(out, s, "source: {src}"),
            Err(error) => panic!("failed to evaluate string literal {src}: {error:?}"),
        }
    })
    .run();
}

/// Property: any char, escaped and embedded as a Rune char literal, evaluates
/// back to the original char.
#[test]
fn char_literal_roundtrip() {
    let context = Context::with_default_modules().expect("failed to build context");

    hegel::Hegel::new(|tc| {
        let c: char = tc.draw(
            hegel::one_of!(
                generators::integers::<u32>().max_value(0x7F),
                generators::integers::<u32>().max_value(0x10_FFFF),
            )
            .map(|n| char::from_u32(n).unwrap_or('\u{FFFD}')),
        );

        // The lexer rejects raw control characters (including DEL and the C1
        // range) inside char literals, so those must use escapes.
        let escaped = match c {
            '\'' => "\\'".to_string(),
            '\\' => "\\\\".to_string(),
            c if c.is_control() => format!("\\u{{{:x}}}", c as u32),
            c => c.to_string(),
        };

        let src = format!("'{escaped}'");

        match eval_result::<char>(&context, &src) {
            Ok(out) => assert_eq!(out, c, "source: {src}"),
            Err(error) => panic!("failed to evaluate char literal {src}: {error:?}"),
        }
    })
    .run();
}
