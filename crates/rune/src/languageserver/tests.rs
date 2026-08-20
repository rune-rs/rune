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
