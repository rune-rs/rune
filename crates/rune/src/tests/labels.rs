//! Where a label ends.
//!
//! A label and a character literal start with the same character, so which one
//! is being read is only settled by what follows. A label ends at the first
//! thing which is not part of one; a character literal ends at the closing
//! quote and may not run past the end of a line.

prelude!();

/// A newline is one of the things a label ends at.
///
/// It was tested for as a control character before it was tested for as the end
/// of a label, so a label written at the end of a line - which is where the last
/// expression of a block is written - was read as a character literal which had
/// never been closed, and the program did not compile.
#[test]
fn a_label_may_end_a_line() {
    let value: i64 = eval(
        "let n = 0;\n\
         'outer: loop {\n\
             n = 1;\n\
             loop {\n\
                 n = 2;\n\
                 break 'outer\n\
             }\n\
         }\n\
         n",
    );

    assert_eq!(value, 2);

    let value: i64 = eval(
        "let n = 0;\n\
         'outer: for i in 0..3 {\n\
             n += 1;\n\
             continue 'outer\n\
         }\n\
         n",
    );

    assert_eq!(value, 3);

    // The label is the value of the block it ends, which is the shape which
    // made this worth having.
    let value: i64 = eval(
        "let a = 'outer: loop {\n\
             break 'outer 7\n\
         };\n\
         a",
    );

    assert_eq!(value, 7);
}

/// A character literal still may not run past the end of a line.
#[test]
fn a_character_literal_may_not_end_a_line() {
    let context = Context::with_default_modules().expect("Failed to build context");
    let mut sources = crate::tests::sources("let a = '\n';\na");
    let mut diagnostics = Diagnostics::new();

    let mut options = Options::default();
    options.script(true);

    let result = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_options(&options)
        .build();

    assert!(result.is_err(), "Should not compile");
}
