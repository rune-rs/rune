use super::Code;

#[test]
fn test_code() {
    let code: Code = serde_json::from_str("-1").unwrap();
    assert_eq!(code, Code::Unknown(-1));
    assert_eq!(serde_json::to_string(&code).unwrap(), "-1");

    let code: Code = serde_json::from_str("-32601").unwrap();
    assert_eq!(code, Code::MethodNotFound);
    assert_eq!(serde_json::to_string(&code).unwrap(), "-32601");
}

use ropey::Rope;

use super::state::StateEncoding;

/// Where a position lands in the document.
///
/// The character is counted in the units the encoding names, and the line has
/// to be measured in the same units - adding a count of UTF-16 code units to a
/// count of characters is only the same thing while everything before it is one
/// code unit, so an emoji anywhere earlier in the document moved every position
/// after it along by one.
#[test]
fn a_position_is_counted_in_the_units_the_encoding_names() {
    let rope = Rope::from_str("let a = \"\u{1F600}\";\nlet b = 2;\n");

    // The `2` on the second line, whatever is on the first.
    for (encoding, character) in [(StateEncoding::Utf16, 8), (StateEncoding::Utf8, 8)] {
        let position = lsp::Position { line: 1, character };

        let at = encoding
            .rope_position(&rope, position)
            .expect("Should resolve");

        assert_eq!(rope.char(at), '2', "{encoding}");
    }

    // And the emoji itself, which is two code units wide in one encoding and
    // four bytes in the other.
    for (encoding, character) in [(StateEncoding::Utf16, 9), (StateEncoding::Utf8, 9)] {
        let position = lsp::Position { line: 0, character };

        let at = encoding
            .rope_position(&rope, position)
            .expect("Should resolve");

        assert_eq!(rope.char(at), '\u{1F600}', "{encoding}");
    }
}

/// A character past the end of the line is the end of that line, which is what
/// the protocol says. Without it a column past the end addressed the next line.
#[test]
fn a_character_past_the_end_of_a_line_is_the_end_of_it() {
    let rope = Rope::from_str("let a = 1;\nlet b = 2;\n");

    for encoding in [StateEncoding::Utf16, StateEncoding::Utf8] {
        for character in [10, 11, 40, u32::MAX] {
            let position = lsp::Position { line: 0, character };

            let at = encoding
                .rope_position(&rope, position)
                .expect("Should resolve");

            assert_eq!(at, 10, "{encoding} at {character}");
        }
    }
}

/// A line the document does not have is an error rather than a panic, which is
/// what lets a change carrying one be reported and skipped instead of stopping
/// the server.
#[test]
fn a_line_past_the_end_is_an_error() {
    let rope = Rope::from_str("let a = 1;\nlet b = 2;\n");

    for encoding in [StateEncoding::Utf16, StateEncoding::Utf8] {
        for line in [3, 99, u32::MAX] {
            let position = lsp::Position { line, character: 0 };

            assert!(
                encoding.rope_position(&rope, position).is_err(),
                "{encoding} line {line} should not resolve"
            );
        }
    }
}

/// What a client sends to ask for `utf-8` positions.
#[test]
fn a_client_can_ask_for_utf8_positions() {
    let params: lsp::InitializeParams = serde_json::from_str(
        r#"{"capabilities": {"general": {"positionEncodings": ["utf-8"]}}, "processId": null}"#,
    )
    .expect("Should deserialize");

    assert!(super::is_utf8(&params));

    let params: lsp::InitializeParams =
        serde_json::from_str(r#"{"capabilities": {}, "processId": null}"#)
            .expect("Should deserialize");

    assert!(!super::is_utf8(&params));
}

/// A position sent back to the client is counted in the units the encoding
/// names, the same way round as when one arrives.
#[test]
fn a_position_sent_back_is_counted_the_same_way() {
    let source = crate::Source::memory("let a = \"\u{1F600}\"; let b = 2;\n").expect("source");

    // The `2`, which is at byte 24 and at code unit 22 of the line.
    let at = source.as_str().find('2').expect("The source has a `2`");

    let (line, character) = source.find_utf16cu_line_column(at);
    assert_eq!((line, character), (0, 22));

    let (line, character) = source.find_utf8_line_column(at);
    assert_eq!((line, character), (0, 24));
}

/// Looking back from where the cursor is, to find what is being completed.
///
/// Where it looks is a byte index and where the cursor is is a character index.
/// Taking one for the other reads the wrong part of the line as soon as the
/// document holds a character wider than a byte, and lands inside one as soon
/// as the cursor is past it - which is not a mistake a `&str` slice survives.
#[test]
fn looking_back_is_counted_in_characters() {
    use ropey::Rope;

    let content = Rope::from_str("let a = \"\u{1F600}\"; let b = a.\n");
    let at = content.len_chars() - 1;

    let (symbol, _) = super::state::looking_back(&content, at)
        .expect("Should not fail")
        .expect("Should find something");

    assert_eq!(symbol, ".");

    // Every position in the document, including the ones inside a character.
    for at in 0..content.len_chars() + 4 {
        super::state::looking_back(&content, at).expect("Should not fail");
    }
}

/// How wide a piece of text is in the units a position is counted in.
///
/// The edit a completion offers covers the symbol being completed, and where
/// that starts used to be worked out by taking the length of the symbol in
/// bytes off a position counted in something else.
#[test]
fn a_width_is_counted_in_the_units_the_encoding_names() {
    for (text, utf16, utf8) in [
        ("foo", 3, 3),
        ("\u{30c6}\u{30b9}\u{30c8}", 3, 9),
        ("\u{1F600}", 2, 4),
        ("", 0, 0),
    ] {
        assert_eq!(StateEncoding::Utf16.width(text), utf16, "{text:?}");
        assert_eq!(StateEncoding::Utf8.width(text), utf8, "{text:?}");
    }
}
