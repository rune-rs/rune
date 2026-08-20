prelude!();

use diagnostics::WarningDiagnosticKind::*;

#[test]
fn test_let_pattern_might_panic() {
    assert_warnings! {
        "let [0, 1, 3] = [];",
        span!(4, 13), LetPatternMightPanic { context: Some(span!(0, 19)), .. }
    };
}

#[test]
fn test_template_without_variables() {
    assert_warnings! {
        "`Hello World`",
        span!(0, 12), TemplateWithoutExpansions { context: Some(span!(0, 13)), .. }
    };
}

/// A warning says what it is on the line which introduces it.
///
/// It said "Warning", leaving whoever read it to find the text on the label
/// below - which an error has never done.
#[test]
fn a_warning_says_what_it_is() {
    fn emitted(source: &str) -> rust_alloc::string::String {
        let context = Context::with_default_modules().expect("Failed to build context");

        let mut sources = crate::tests::sources(source);
        let mut diagnostics = Diagnostics::new();

        let mut options = Options::default();
        options.script(true);

        let _ = crate::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .with_options(&options)
            .build();

        let mut buffer = crate::termcolor::Buffer::no_color();

        diagnostics
            .emit(&mut buffer, &sources)
            .expect("Failed to emit");

        rust_alloc::string::String::from_utf8(buffer.into_inner()).expect("Output should be utf-8")
    }

    for (source, expected) in [
        ("fn f() { 1 }", "warning: Not used"),
        ("use std::collections::HashMap;", "warning: Not used"),
        ("fn f() { return 1; 2 } f();", "warning: Unreachable code"),
        (
            "`Hello World`",
            "warning: Using a template string without expansions",
        ),
    ] {
        let out = emitted(source);

        assert!(out.contains(expected), "{source}: {out}");
        assert!(!out.contains("warning: Warning"), "{source}: {out}");
    }
}
