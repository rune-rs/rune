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
    // The `.`, which is the last thing on the line.
    let at = content.len_chars() - 2;

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

/// What is being completed is on the line the cursor is on.
///
/// A line ending delimits a symbol like anything else does. Without that, a
/// line whose text reaches back to a delimiter on an earlier line was taken to
/// be part of the symbol, and the symbol then matched nothing.
#[test]
fn looking_back_stops_at_the_line() {
    use ropey::Rope;

    let content = Rope::from_str("let a = 1\nbar\n");
    // The last `r`, which is the last thing on the line.
    let at = content.len_chars() - 2;

    let (symbol, _) = super::state::looking_back(&content, at)
        .expect("Should not fail")
        .expect("Should find something");

    assert_eq!(symbol.trim_start_matches('\n'), "bar");
}

/// A path with something awkward in it survives the trip to a URL and back.
///
/// This is how a file the editor names is found on disk, so a path which does
/// not come back is a file the server cannot open.
#[test]
fn a_path_survives_being_made_into_a_url() {
    use std::path::{Path, PathBuf};

    for path in [
        "/tmp/plain.rn",
        "/tmp/with a space.rn",
        "/tmp/with#hash.rn",
        "/tmp/with%percent.rn",
        "/tmp/with?question.rn",
        "/tmp/with'quote.rn",
        "/tmp/with\"quote.rn",
        "/tmp/\u{e5}\u{e4}\u{f6}.rn",
        "/tmp/\u{1F600}.rn",
        "/tmp/with{brace}.rn",
        "/tmp/with backtick`.rn",
        "/tmp/a/b/c.rn",
    ] {
        let url = super::url::from_file_path(path).expect("Should become a url");

        let back: PathBuf = url
            .to_file_path()
            .unwrap_or_else(|()| panic!("{path}: {url} should become a path"));

        assert_eq!(back, Path::new(path), "{url}");
    }
}

/// A request the server cannot serve is answered with an error.
///
/// Ending the loop over it instead took the whole session down - every open
/// file, every diagnostic - and a position the document does not have is just
/// a client which is a keystroke ahead of us.
#[tokio::test]
async fn a_request_which_fails_does_not_stop_the_server() {
    /// Frame a message the way the protocol does.
    fn frame(out: &mut rust_alloc::vec::Vec<u8>, message: &str) {
        use std::io::Write;
        write!(out, "Content-Length: {}\r\n\r\n{message}", message.len()).expect("Should write");
    }

    /// Every response the server wrote, by the id it carries.
    fn responses(mut data: &[u8]) -> rust_alloc::vec::Vec<serde_json::Value> {
        let mut out = rust_alloc::vec::Vec::new();

        while let Some(at) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            let header = std::str::from_utf8(&data[..at]).expect("Header should be utf-8");

            let len: usize = header
                .split_once(':')
                .expect("Header should have a length")
                .1
                .trim()
                .parse()
                .expect("Length should be a number");

            let body = &data[at + 4..at + 4 + len];
            out.push(serde_json::from_slice(body).expect("Body should be json"));
            data = &data[at + 4 + len..];
        }

        out
    }

    let uri = "file:///tmp/does-not-stop.rn";

    let mut input = rust_alloc::vec::Vec::new();

    frame(
        &mut input,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{},"processId":null}}"#,
    );
    frame(
        &mut input,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );
    frame(
        &mut input,
        &rust_alloc::format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri}","languageId":"rune","version":1,"text":"let a = 1;\n"}}}}}}"#
        ),
    );

    // A line the document does not have.
    for (id, method) in [
        (2, "textDocument/completion"),
        (3, "textDocument/definition"),
    ] {
        frame(
            &mut input,
            &rust_alloc::format!(
                r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":99,"character":0}}}}}}"#
            ),
        );
    }

    // Params which are not what the method takes.
    frame(
        &mut input,
        r#"{"jsonrpc":"2.0","id":4,"method":"textDocument/completion","params":{"nonsense":true}}"#,
    );

    // And something which should still be served afterwards.
    frame(
        &mut input,
        &rust_alloc::format!(
            r#"{{"jsonrpc":"2.0","id":5,"method":"textDocument/formatting","params":{{"textDocument":{{"uri":"{uri}"}},"options":{{"tabSize":4,"insertSpaces":true}}}}}}"#
        ),
    );

    let mut output = rust_alloc::vec::Vec::new();

    super::builder()
        .with_input(&input[..])
        .with_output(&mut output)
        .build()
        .expect("Should build")
        .run()
        .await
        .expect("The server should not stop over a request it cannot serve");

    let responses = responses(&output);

    let find = |id: i64| {
        responses
            .iter()
            .find(|m| m.get("id").and_then(serde_json::Value::as_i64) == Some(id))
            .unwrap_or_else(|| panic!("Should have answered {id}: {responses:?}"))
    };

    for id in [2, 3, 4] {
        let response = find(id);
        assert!(response.get("error").is_some(), "{id}: {response}");
    }

    let response = find(5);
    assert!(response.get("error").is_none(), "5: {response}");
}
