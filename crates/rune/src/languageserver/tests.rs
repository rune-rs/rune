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

/// A whole language server, driven over a duplex stream.
///
/// Everything the server does past parsing a position lives behind the loop in
/// `run`, so this is the only way to reach it. The stream is a real one rather
/// than a slice written up front, because the server only gets to rebuild - and
/// so only gets to report diagnostics or resolve a definition - while it is
/// waiting on its input.
struct Session {
    /// What to write, split where the client has to wait for the server before
    /// it can sensibly say the next thing.
    stages: rust_alloc::vec::Vec<(rust_alloc::vec::Vec<u8>, Option<rust_alloc::string::String>)>,
    id: i64,
}

impl Session {
    /// Start a session, having initialized it.
    fn new() -> Self {
        Session::with_capabilities(serde_json::json!({}))
    }

    /// Start a session which counts positions in bytes.
    fn utf8() -> Self {
        Session::with_capabilities(serde_json::json!({"general": {"positionEncodings": ["utf-8"]}}))
    }

    fn with_capabilities(capabilities: serde_json::Value) -> Self {
        let mut this = Session {
            stages: rust_alloc::vec::from_elem((rust_alloc::vec::Vec::new(), None), 1),
            id: 0,
        };

        this.request(
            "initialize",
            serde_json::json!({"capabilities": capabilities, "processId": null}),
        );

        this.notify("initialized", serde_json::json!({}));
        this
    }

    /// Frame a message the way the protocol does.
    fn frame(&mut self, message: &serde_json::Value) {
        use std::io::Write;

        let body = serde_json::to_string(message).expect("Should serialize");

        let (bytes, _) = self.stages.last_mut().expect("There is always a stage");

        write!(bytes, "Content-Length: {}\r\n\r\n{body}", body.len()).expect("Should write");
    }

    /// Wait for the server to have diagnosed `uri` before saying anything more.
    ///
    /// The server only rebuilds while it is idle, so a question about what a
    /// document means - where a name is defined, what it is - has to be asked
    /// after it has caught up, the way a client asks after seeing the file
    /// diagnosed.
    fn after_diagnostics(&mut self, uri: &str) {
        let (_, wait) = self.stages.last_mut().expect("There is always a stage");
        *wait = Some(rust_alloc::string::String::from(uri));
        self.stages.push((rust_alloc::vec::Vec::new(), None));
    }

    fn notify(&mut self, method: &str, params: serde_json::Value) {
        self.frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }));
    }

    /// Send a request, returning the id it was sent with.
    fn request(&mut self, method: &str, params: serde_json::Value) -> i64 {
        self.id += 1;
        let id = self.id;

        self.frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));

        id
    }

    /// Open a document.
    fn open(&mut self, uri: &str, text: &str) {
        self.notify(
            "textDocument/didOpen",
            serde_json::json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "rune",
                    "version": 1,
                    "text": text,
                }
            }),
        );
    }

    /// Run the server until `ready` is satisfied by what it has written, then
    /// close its input and return everything it wrote.
    ///
    /// Waiting on what the server said rather than on the end of the stream is
    /// what gives it the chance to rebuild, since that only happens while it is
    /// idle.
    #[track_caller]
    fn run(
        self,
        ready: impl Fn(&[serde_json::Value]) -> bool,
    ) -> rust_alloc::vec::Vec<serde_json::Value> {
        use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Should build a runtime");

        let messages = runtime.block_on(async move {
            let (mut client, server) = tokio::io::duplex(1 << 20);
            let (input, output) = tokio::io::split(server);

            let server = super::builder()
                .with_input(input)
                .with_output(output)
                .build()
                .expect("Should build");

            let handle = tokio::spawn(server.run());

            let mut buf = rust_alloc::vec::Vec::new();
            let mut chunk = [0u8; 4096];
            let mut messages = rust_alloc::vec::Vec::new();

            // A generous ceiling. Nothing here waits on anything but the
            // server, so reaching it means the server never answered.
            let deadline = tokio::time::Duration::from_secs(60);

            let mut pump = async |buf: &mut rust_alloc::vec::Vec<u8>,
                                  messages: &mut rust_alloc::vec::Vec<serde_json::Value>,
                                  client: &mut tokio::io::DuplexStream| {
                let read = tokio::time::timeout(deadline, client.read(&mut chunk))
                    .await
                    .expect("The server should answer")
                    .expect("Should read");

                buf.extend_from_slice(&chunk[..read]);

                while let Some(message) = take_frame(buf) {
                    messages.push(message);
                }

                read
            };

            for (bytes, wait) in self.stages {
                client.write_all(&bytes).await.expect("Should write");

                let Some(uri) = wait else {
                    continue;
                };

                while !notifications(&messages, "textDocument/publishDiagnostics")
                    .any(|m| m["params"]["uri"] == uri.as_str())
                {
                    if pump(&mut buf, &mut messages, &mut client).await == 0 {
                        break;
                    }
                }
            }

            while !ready(&messages) {
                if pump(&mut buf, &mut messages, &mut client).await == 0 {
                    break;
                }
            }

            // Closing the input is what ends the loop the server runs.
            drop(client);

            handle
                .await
                .expect("The server should not panic")
                .expect("The server should run to the end of the stream");

            messages
        });

        messages
    }
}

/// Take one framed message off the front of `buf`, if a whole one is there.
fn take_frame(buf: &mut rust_alloc::vec::Vec<u8>) -> Option<serde_json::Value> {
    let at = buf.windows(4).position(|w| w == b"\r\n\r\n")?;

    let header = std::str::from_utf8(&buf[..at]).expect("Header should be utf-8");

    let len: usize = header
        .split_once(':')
        .expect("Header should have a length")
        .1
        .trim()
        .parse()
        .expect("Length should be a number");

    if buf.len() < at + 4 + len {
        return None;
    }

    let message = serde_json::from_slice(&buf[at + 4..at + 4 + len]).expect("Body should be json");
    buf.drain(..at + 4 + len);
    Some(message)
}

/// The response to the request which was sent with the given id.
#[track_caller]
fn response(messages: &[serde_json::Value], id: i64) -> &serde_json::Value {
    messages
        .iter()
        .find(|m| m.get("id").and_then(serde_json::Value::as_i64) == Some(id))
        .unwrap_or_else(|| panic!("Should have answered {id}: {messages:?}"))
}

/// Whether the request sent with the given id has been answered.
fn answered(messages: &[serde_json::Value], id: i64) -> bool {
    messages
        .iter()
        .any(|m| m.get("id").and_then(serde_json::Value::as_i64) == Some(id))
}

/// Every notification of the given method.
fn notifications<'a>(
    messages: &'a [serde_json::Value],
    method: &'a str,
) -> impl Iterator<Item = &'a serde_json::Value> {
    messages
        .iter()
        .filter(move |m| m.get("method").and_then(serde_json::Value::as_str) == Some(method))
}

/// A request the server cannot serve is answered with an error.
///
/// Ending the loop over it instead took the whole session down - every open
/// file, every diagnostic - and a position the document does not have is just
/// a client which is a keystroke ahead of us.
#[test]
fn a_request_which_fails_does_not_stop_the_server() {
    let uri = "file:///tmp/does-not-stop.rn";

    let mut s = Session::new();
    s.open(uri, "let a = 1;\n");

    // A line the document does not have.
    let out_of_range = [
        s.request(
            "textDocument/completion",
            serde_json::json!({
                "textDocument": {"uri": uri},
                "position": {"line": 99, "character": 0},
            }),
        ),
        s.request(
            "textDocument/definition",
            serde_json::json!({
                "textDocument": {"uri": uri},
                "position": {"line": 99, "character": 0},
            }),
        ),
        // Params which are not what the method takes.
        s.request(
            "textDocument/completion",
            serde_json::json!({"nonsense": true}),
        ),
    ];

    // And something which should still be served afterwards.
    let served = s.request(
        "textDocument/formatting",
        serde_json::json!({
            "textDocument": {"uri": uri},
            "options": {"tabSize": 4, "insertSpaces": true},
        }),
    );

    let messages = s.run(|m| answered(m, served));

    for id in out_of_range {
        let m = response(&messages, id);
        assert!(m.get("error").is_some(), "{id}: {m}");
    }

    let m = response(&messages, served);
    assert!(m.get("error").is_none(), "{served}: {m}");
}

/// What a document says is reported back as diagnostics.
#[test]
fn a_document_is_diagnosed_when_it_is_opened() {
    let uri = "file:///tmp/diagnosed.rn";

    let mut s = Session::new();
    s.open(uri, "pub fn main() { let a = 1 }\n");

    let messages = s.run(|m| {
        notifications(m, "textDocument/publishDiagnostics").any(|m| m["params"]["uri"] == uri)
    });

    let published = notifications(&messages, "textDocument/publishDiagnostics")
        .find(|m| m["params"]["uri"] == uri)
        .unwrap_or_else(|| panic!("Should have diagnosed {uri}: {messages:?}"));

    let diagnostics = published["params"]["diagnostics"]
        .as_array()
        .expect("Diagnostics should be a list");

    assert!(
        diagnostics
            .iter()
            .any(|d| d["message"].as_str().is_some_and(|m| m.contains("`;`"))),
        "{published}"
    );

    // And the range it points at is inside the document.
    let at = &diagnostics[0]["range"]["start"];
    assert_eq!(at["line"], 0, "{published}");
}

/// A document which is edited is diagnosed as it is after the edit.
///
/// The server keeps its own copy and applies the ranges a client sends, so an
/// edit which is dropped or applied in the wrong place shows up here as a
/// diagnostic which does not match what the client thinks the file says.
#[test]
fn an_edit_is_reflected_in_what_is_reported() {
    let uri = "file:///tmp/edited.rn";

    let mut s = Session::new();
    s.open(uri, "pub fn main() { let a = 1 }\n");

    // Use the binding, which is what the warning above is about.
    s.notify(
        "textDocument/didChange",
        serde_json::json!({
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{
                "range": {
                    "start": {"line": 0, "character": 25},
                    "end": {"line": 0, "character": 25},
                },
                "text": "; a",
            }],
        }),
    );

    let formatted = s.request(
        "textDocument/formatting",
        serde_json::json!({
            "textDocument": {"uri": uri},
            "options": {"tabSize": 4, "insertSpaces": true},
        }),
    );

    let messages = s.run(|m| answered(m, formatted));

    let edits = response(&messages, formatted)["result"]
        .as_array()
        .expect("Formatting should produce edits")
        .clone();

    let text = edits[0]["newText"].as_str().expect("An edit is text");

    assert!(text.contains("let a = 1;"), "{text}");
    assert!(text.contains("\n    a\n"), "{text}");
}

/// What is offered for a partial name.
#[test]
fn a_completion_offers_what_the_prefix_names() {
    let uri = "file:///tmp/completed.rn";

    let mut s = Session::new();
    s.open(uri, "pub fn main() { let v = []; v.ins }\n");

    // Just past the `ins`.
    let completed = s.request(
        "textDocument/completion",
        serde_json::json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 32},
        }),
    );

    let messages = s.run(|m| answered(m, completed));
    let result = &response(&messages, completed)["result"];

    let items = result
        .as_array()
        .unwrap_or_else(|| panic!("Completion should produce a list: {result}"));

    let insert = items
        .iter()
        .find(|i| i["label"] == "insert")
        .unwrap_or_else(|| panic!("Should offer `insert`: {result}"));

    // The edit covers the symbol which was typed rather than sitting after it,
    // which is what puts `v.insert` in the document instead of `v.insinsert`.
    let start = &insert["textEdit"]["range"]["start"];
    assert_eq!(start["line"], 0);
    assert_eq!(start["character"], 29, "{insert}");
}

/// Where a name is defined.
#[test]
fn a_definition_is_where_the_name_was_declared() {
    let uri = "file:///tmp/defined.rn";

    let mut s = Session::new();
    s.open(uri, "fn helper() { 1 }\n\npub fn main() { helper() }\n");
    s.after_diagnostics(uri);

    // The `helper` on the last line.
    let defined = s.request(
        "textDocument/definition",
        serde_json::json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 18},
        }),
    );

    let messages = s.run(|m| answered(m, defined));
    let result = &response(&messages, defined)["result"];

    assert_eq!(result["uri"], uri, "{result}");
    assert_eq!(result["range"]["start"]["line"], 0, "{result}");
}

/// A document is closed and no longer reported on.
#[test]
fn a_closed_document_is_dropped() {
    let uri = "file:///tmp/closed.rn";

    let mut s = Session::new();
    s.open(uri, "pub fn main() { let a = 1 }\n");

    s.notify(
        "textDocument/didClose",
        serde_json::json!({"textDocument": {"uri": uri}}),
    );

    // Asking about it now is a request the server cannot serve, which it
    // answers rather than stopping over.
    let after = s.request(
        "textDocument/formatting",
        serde_json::json!({
            "textDocument": {"uri": uri},
            "options": {"tabSize": 4, "insertSpaces": true},
        }),
    );

    let messages = s.run(|m| answered(m, after));
    let m = response(&messages, after);

    assert!(
        m.get("error").is_some() || m["result"].is_null(),
        "A closed document has nothing to format: {m}"
    );
}

/// A position is read in the units the client asked for, all the way through.
///
/// The unit tests above cover the conversion; this covers the whole trip, which
/// is where it was wrong in both directions: the column of the symbol being
/// completed was measured in bytes and taken off a position counted in code
/// units, so an emoji earlier on the line moved the edit.
#[test]
fn a_completion_lands_in_the_same_place_in_either_encoding() {
    // The `v.ins` is preceded by an emoji, which is two code units and four
    // bytes, so the two encodings name the same place with different numbers.
    let text = "pub fn main() { let e = \"\u{1F600}\"; let v = []; v.ins }\n";

    // Where `ins` starts, counted each way.
    let prefix = text.split_once("v.ins").expect("The line has a `v.ins`").0;
    let utf16 = prefix.chars().map(char::len_utf16).sum::<usize>() + 2;
    let utf8 = prefix.len() + 2;

    for (utf8_mode, at, want_start) in [(false, utf16 + 3, utf16), (true, utf8 + 3, utf8)] {
        let uri = "file:///tmp/encoded.rn";

        let mut s = if utf8_mode {
            Session::utf8()
        } else {
            Session::new()
        };

        s.open(uri, text);

        let completed = s.request(
            "textDocument/completion",
            serde_json::json!({
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": at},
            }),
        );

        let messages = s.run(|m| answered(m, completed));
        let result = &response(&messages, completed)["result"];

        let items = result
            .as_array()
            .unwrap_or_else(|| panic!("utf8={utf8_mode}: should produce a list: {result}"));

        let insert = items
            .iter()
            .find(|i| i["label"] == "insert")
            .unwrap_or_else(|| panic!("utf8={utf8_mode}: should offer `insert`: {result}"));

        assert_eq!(
            insert["textEdit"]["range"]["start"]["character"], want_start,
            "utf8={utf8_mode}: {insert}"
        );

        assert_eq!(
            insert["textEdit"]["range"]["end"]["character"], at,
            "utf8={utf8_mode}: {insert}"
        );
    }
}
