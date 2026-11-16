prelude!();

use crate::highlight::{highlight, Class};

fn classes(source: &str) -> Vec<(Class, &str)> {
    highlight(source)
        .unwrap()
        .into_iter()
        .map(|t| (t.class, &source[t.span.range()]))
        .collect()
}

fn assert_covered(source: &str) {
    let mut pos = 0;

    for token in highlight(source).unwrap() {
        assert_eq!(token.span.start.into_usize(), pos, "gap in {source:?}");
        assert!(
            token.span.end.into_usize() > pos,
            "empty token in {source:?}"
        );
        pos = token.span.end.into_usize();
    }

    assert_eq!(pos, source.len(), "short cover of {source:?}");
}

/// A highlighter which skips or repeats a byte would colour the wrong text in
/// an editor.
#[test]
fn every_byte_is_covered_exactly_once() {
    for source in [
        "",
        "fn main() { let x = 1 + 2; }",
        "#!/usr/bin/rune\nfn f() {}",
        "/// doc\n#[test]\nfn é() { println!(\"héllo\"); `a${b}c` }",
        "\"unterminated",
        "`unterminated ${",
        "/* never closed",
        "'a' b'b' 'label 0xff 1.5e3 $ ~ \u{1F600}",
    ] {
        assert_covered(source);
    }
}

/// An editor gets text which does not compile on every keystroke, and must
/// still be able to colour what came before the mistake.
#[test]
fn an_unterminated_string_or_template_still_yields_tokens() {
    let tokens = classes("let s = \"abc");
    assert_eq!(tokens[0], (Class::Keyword, "let"));
    assert_eq!(tokens.last(), Some(&(Class::Error, "\"abc")));

    let tokens = classes("let t = `abc ${x");
    assert_eq!(tokens[0], (Class::Keyword, "let"));
    assert_eq!(tokens.last().unwrap().0, Class::Error);
    assert_covered("let t = `abc ${x");
}

/// Pins the mapping from the lexer's tokens to the classes.
#[test]
fn a_sample_with_each_class_maps_correctly() {
    let source = "// c\n/// d\n#[attr(1)] fn f() { println!(\"s\", 'c', b'b', 12, `t`, x + y); }";
    let tokens = classes(source);

    let has = |class, text| tokens.contains(&(class, text));

    assert!(has(Class::Comment, "// c"));
    assert!(has(Class::DocComment, "/// d"));
    assert!(has(Class::Attribute, "#"));
    assert!(has(Class::Attribute, "attr"));
    assert!(has(Class::Attribute, "]"));
    assert!(has(Class::Keyword, "fn"));
    assert!(has(Class::Ident, "f"));
    assert!(has(Class::Macro, "println"));
    assert!(has(Class::Punctuation, "("));
    assert!(has(Class::Punctuation, ","));
    assert!(has(Class::String, "\"s\""));
    assert!(has(Class::Char, "'c'"));
    assert!(has(Class::Byte, "b'b'"));
    assert!(has(Class::Number, "12"));
    assert!(has(Class::Template, "`t`"));
    assert!(has(Class::Operator, "+"));
    assert!(has(Class::Whitespace, " "));
    assert!(!has(Class::Attribute, "fn"));
}

/// Four slashes is a separator rather than documentation.
#[test]
fn a_comment_is_a_doc_comment_only_when_it_is_one() {
    assert_eq!(classes("//// x")[0].0, Class::Comment);
    assert_eq!(classes("//! x")[0].0, Class::DocComment);
    assert_eq!(classes("/** x */")[0].0, Class::DocComment);
    assert_eq!(classes("/**/")[0].0, Class::Comment);
}

/// The attribute scan and the macro lookahead walk a flat list, so deep
/// nesting must not overflow the native stack.
#[test]
fn deep_nesting_is_not_recursed_over() {
    let source = format!("#[{}{}]", "(".repeat(200_000), ")".repeat(200_000));
    assert_covered(&source);

    let source = format!("{}{}", "[".repeat(200_000), "]".repeat(200_000));
    assert_covered(&source);
}
