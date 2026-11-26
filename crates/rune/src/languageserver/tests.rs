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

        let url_str = url.as_str();

        let back: PathBuf = super::url::to_file_path(&url)
            .unwrap_or_else(|e| panic!("{path}: {url_str} should become a path: {e}"));

        assert_eq!(back, Path::new(path), "{url_str}");
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
            stages: rust_alloc::vec![(rust_alloc::vec::Vec::new(), None)],
            id: 0,
        };

        this.request(
            "initialize",
            serde_json::json!({"capabilities": capabilities, "processId": null}),
        );

        this.notify("initialized", serde_json::json!({}));
        this
    }

    /// Start a session whose root has no `Rune.toml` in it, which is what an
    /// editor opened on a loose folder of scripts sends.
    fn without_manifest() -> Self {
        let mut this = Session {
            stages: rust_alloc::vec![(rust_alloc::vec::Vec::new(), None)],
            id: 0,
        };

        this.request(
            "initialize",
            serde_json::json!({
                "capabilities": {},
                "processId": null,
                "rootUri": "file:///tmp/rune-languageserver-no-manifest-here",
            }),
        );

        this.notify("initialized", serde_json::json!({}));
        this
    }

    /// Ask for `uri` to be formatted, returning the id it was asked with.
    fn format(&mut self, uri: &str) -> i64 {
        self.request(
            "textDocument/formatting",
            serde_json::json!({
                "textDocument": {"uri": uri},
                "options": {"tabSize": 4, "insertSpaces": true},
            }),
        )
    }

    /// Ask for part of `uri` to be formatted, returning the id it was asked
    /// with.
    fn format_range(&mut self, uri: &str, start: (u32, u32), end: (u32, u32)) -> i64 {
        self.request(
            "textDocument/rangeFormatting",
            serde_json::json!({
                "textDocument": {"uri": uri},
                "range": {
                    "start": {"line": start.0, "character": start.1},
                    "end": {"line": end.0, "character": end.1},
                },
                "options": {"tabSize": 4, "insertSpaces": true},
            }),
        )
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

/// Every message which answers a request, whatever its id.
fn responses(messages: &[serde_json::Value]) -> impl Iterator<Item = &serde_json::Value> {
    messages
        .iter()
        .filter(|m| m.get("result").is_some() || m.get("error").is_some())
}

/// A cancelled request is not answered a second time, and the cancellation is
/// not answered at all.
///
/// Clients send `$/cancelRequest` routinely. It used to fall through to the
/// catch-all, which answered it with a `MethodNotFound` error carrying an id of
/// `null` - a response to nothing, written for every cancellation.
#[test]
fn a_cancellation_is_not_answered() {
    let uri = "file:///tmp/cancelled.rn";

    let mut s = Session::new();
    s.open(uri, "let a = 1;\n");

    let finished = s.request(
        "textDocument/formatting",
        serde_json::json!({
            "textDocument": {"uri": uri},
            "options": {"tabSize": 4, "insertSpaces": true},
        }),
    );

    // One for a request which has been served, and one for an id the server
    // has never seen.
    s.notify("$/cancelRequest", serde_json::json!({"id": finished}));
    s.notify("$/cancelRequest", serde_json::json!({"id": 9999}));
    s.notify("$/cancelRequest", serde_json::json!({"id": "unknown"}));

    let served = s.request(
        "textDocument/formatting",
        serde_json::json!({
            "textDocument": {"uri": uri},
            "options": {"tabSize": 4, "insertSpaces": true},
        }),
    );

    let messages = s.run(|m| answered(m, served));

    let m = response(&messages, served);
    assert!(m.get("error").is_none(), "{served}: {m}");

    // The initialize request and the two formatting requests, once each.
    let ids = responses(&messages)
        .map(|m| m["id"].clone())
        .collect::<rust_alloc::vec::Vec<_>>();

    assert_eq!(ids, [1, finished, served], "{messages:?}");

    // And nothing was logged about it either.
    assert!(
        !notifications(&messages, "window/logMessage").any(|m| m["params"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("cancelRequest"))),
        "{messages:?}"
    );
}

/// A notification the server does not know is not answered.
///
/// A notification has no id, so an error for one is a response to nothing.
/// The protocol has a `$/` notification ignored, and any other is at most
/// something to log.
#[test]
fn an_unknown_notification_is_not_answered() {
    let mut s = Session::new();

    s.notify("$/setTrace", serde_json::json!({"value": "verbose"}));
    s.notify("$/somethingNew", serde_json::json!({}));
    s.notify("custom/notification", serde_json::json!({"a": 1}));

    let served = s.request("shutdown", serde_json::Value::Null);

    let messages = s.run(|m| answered(m, served));

    let m = response(&messages, served);
    assert!(m.get("error").is_none(), "{served}: {m}");

    assert!(
        responses(&messages).all(|m| !m["id"].is_null()),
        "{messages:?}"
    );

    assert_eq!(responses(&messages).count(), 2, "{messages:?}");
}

/// A request the server does not know is still answered, with
/// `MethodNotFound`, since a client waits on every request it sends.
#[test]
fn an_unknown_request_is_answered_with_method_not_found() {
    let mut s = Session::new();

    let unknown = s.request("custom/request", serde_json::json!({}));
    let dollar = s.request("$/unknownRequest", serde_json::json!({}));

    let messages = s.run(|m| answered(m, unknown) && answered(m, dollar));

    for id in [unknown, dollar] {
        let m = response(&messages, id);
        assert_eq!(m["error"]["code"], -32601, "{id}: {m}");
    }
}

/// Apply the edits a formatting request was answered with to `text`, the way
/// a client does, counting positions in UTF-16 code units.
#[track_caller]
fn apply_edits(text: &str, result: &serde_json::Value) -> rust_alloc::string::String {
    /// The byte offset of a position, clamped to the line the way the
    /// protocol says a client clamps it.
    fn offset(text: &str, position: &serde_json::Value) -> usize {
        let line = position["line"].as_u64().expect("A line") as usize;
        let character = position["character"].as_u64().expect("A character") as usize;

        let mut at = 0;

        for _ in 0..line {
            at += text[at..].find('\n').expect("The line should exist") + 1;
        }

        let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
        let mut units = 0;

        for (n, c) in text[at..end].char_indices() {
            if units >= character {
                return at + n;
            }

            units += c.len_utf16();
        }

        end
    }

    let edits = result
        .as_array()
        .unwrap_or_else(|| panic!("Formatting should produce edits: {result}"));

    let mut text = rust_alloc::string::String::from(text);

    // Edits do not overlap, so applying them back to front keeps the positions
    // of the ones still to apply where they were.
    for edit in edits.iter().rev() {
        let start = offset(&text, &edit["range"]["start"]);
        let end = offset(&text, &edit["range"]["end"]);
        let new = edit["newText"].as_str().expect("An edit is text");
        text.replace_range(start..end, new);
    }

    text
}

/// What `rune fmt` makes of `text`.
fn rune_fmt(text: &str) -> rust_alloc::string::String {
    let options = crate::Options::default();
    let mut diagnostics = crate::Diagnostics::new();

    crate::fmt::layout_source_with(text, crate::SourceId::EMPTY, &options, &mut diagnostics)
        .expect("Should format")
        .into_std()
}

/// Formatting a document gives what `rune fmt` gives, with or without a
/// `Rune.toml` around it.
///
/// A loose script is the first thing most people open, and the edit covers
/// the document up to where it really ends rather than a position past it,
/// which not every client clamps.
#[test]
fn a_document_is_formatted_the_way_rune_fmt_formats_it() {
    let text = "pub fn main(){let e=\"\u{1F600}\";\nlet a   =  1;a}";
    let want = rune_fmt(text);
    assert_ne!(want, text, "The test needs something to format");

    for manifest in [false, true] {
        let uri = "file:///tmp/formatted.rn";

        let mut s = if manifest {
            Session::new()
        } else {
            Session::without_manifest()
        };

        s.open(uri, text);
        let formatted = s.format(uri);

        let messages = s.run(|m| answered(m, formatted));
        let result = &response(&messages, formatted)["result"];

        assert_eq!(apply_edits(text, result), want, "manifest={manifest}");

        // The last line holds `let a   =  1;a}`.
        let end = &result[0]["range"]["end"];
        assert_eq!(end["line"], 1, "{result}");
        assert_eq!(end["character"], 15, "{result}");
    }
}

/// Formatting a document which the formatter cannot lay out is an error.
///
/// Answering with nothing looked, to the person who asked, like the formatter
/// had run and found nothing to change. And whatever the formatter makes of
/// its input, the session carries on: there is no supervisor to start another.
#[test]
fn a_document_which_cannot_be_formatted_is_an_error() {
    // Nested deeper than `max-depth`, which is a limit rather than a stack the
    // formatter could overflow.
    let deep = "(".repeat(70_000);

    let cases = [
        (
            "file:///tmp/unparseable.rn",
            "pub fn main() { let a = ; }\n",
        ),
        ("file:///tmp/unterminated.rn", "pub fn main() { \"abc }\n"),
        ("file:///tmp/deep.rn", deep.as_str()),
    ];

    let mut s = Session::new();
    let mut failing = rust_alloc::vec::Vec::new();

    for (uri, text) in cases {
        s.open(uri, text);
        failing.push(s.format(uri));
    }

    // The same as a selection, all of it.
    for (uri, _) in &cases[..2] {
        failing.push(s.format_range(uri, (0, 0), (1, 0)));
    }

    let deep_range = "file:///tmp/deep-range.rn";
    s.open(deep_range, &format!("{deep}\n"));
    failing.push(s.format_range(deep_range, (0, 0), (1, 0)));

    // A range which cuts a string in half.
    let cut = "file:///tmp/cut.rn";
    s.open(cut, "let a = \"hello world\";\n");
    failing.push(s.format_range(cut, (0, 0), (0, 13)));

    // And something which should still be served afterwards.
    let ok = "file:///tmp/still-served.rn";
    s.open(ok, "let a=1;\n");
    let served = s.format(ok);

    let messages = s.run(|m| answered(m, served));

    for id in failing {
        let m = response(&messages, id);

        let message = m["error"]["message"]
            .as_str()
            .unwrap_or_else(|| panic!("{id}: should be an error: {m}"));

        assert!(message.starts_with("textDocument/"), "{id}: {m}");
    }

    let m = response(&messages, served);
    assert_eq!(
        apply_edits("let a=1;\n", &m["result"]),
        "let a = 1;\n",
        "{m}"
    );
}

/// An error says where in the document the formatter gave up.
#[test]
fn a_formatting_error_says_where_it_is() {
    let uri = "file:///tmp/located.rn";

    let mut s = Session::new();
    s.open(uri, "pub fn main() {\n    let a = ;\n}\n");
    let formatted = s.format(uri);

    let messages = s.run(|m| answered(m, formatted));
    let m = response(&messages, formatted);
    let message = m["error"]["message"]
        .as_str()
        .unwrap_or_else(|| panic!("{m}"));

    assert!(message.contains(": 2:"), "{message}");
}

/// Formatting a selection replaces what was selected and leaves it where it
/// was.
///
/// What is selected is laid out as though it were a file of its own, which put
/// lines selected inside a function at the left margin and joined the last of
/// them onto the line after the selection.
#[test]
fn a_selection_is_formatted_in_place() {
    let text = "pub fn main() {\n    let a   =  1;\n    if a {\n    let b=2;\n    }\n    a\n}\n";

    // From the start of the second line to the start of the sixth, so the
    // selection ends with the line ending before `a`.
    let mut s = Session::new();
    let uri = "file:///tmp/selected.rn";
    s.open(uri, text);
    let whole_lines = s.format_range(uri, (1, 0), (5, 0));

    // A selection which starts and ends partway along a line.
    let mid = "file:///tmp/mid-line.rn";
    let mid_text = "pub fn main() {\n    let a = 1+2  ; a\n}\n";
    s.open(mid, mid_text);
    let mid_line = s.format_range(mid, (1, 12), (1, 15));

    let messages = s.run(|m| answered(m, whole_lines) && answered(m, mid_line));

    let result = &response(&messages, whole_lines)["result"];
    assert_eq!(result[0]["range"]["start"]["line"], 1, "{result}");
    assert_eq!(result[0]["range"]["end"]["line"], 5, "{result}");

    assert_eq!(
        apply_edits(text, result),
        "pub fn main() {\n    let a = 1;\n    if a {\n        let b = 2;\n    }\n    a\n}\n",
    );

    let result = &response(&messages, mid_line)["result"];
    assert_eq!(result[0]["range"]["start"]["character"], 12, "{result}");
    assert_eq!(result[0]["range"]["end"]["character"], 15, "{result}");
    assert_eq!(
        apply_edits(mid_text, result),
        "pub fn main() {\n    let a = 1 + 2  ; a\n}\n",
    );
}

/// Start a session rooted in the fixture package `name` of the dependency
/// tests, returning it along with the URL of the package's `src/main.rn`.
fn dependency_session(name: &str) -> (Session, rust_alloc::string::String) {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/tests/dependencies")
        .join(name)
        .canonicalize()
        .expect("Fixture should exist");

    let root_uri = super::url::from_file_path(&root).expect("Should be a url");
    let main_uri = super::url::from_file_path(root.join("src/main.rn")).expect("Should be a url");

    let mut this = Session {
        stages: rust_alloc::vec![(rust_alloc::vec::Vec::new(), None)],
        id: 0,
    };

    this.request(
        "initialize",
        serde_json::json!({
            "capabilities": {},
            "processId": null,
            "rootUri": root_uri.as_str(),
        }),
    );

    this.notify("initialized", serde_json::json!({}));
    (this, main_uri.as_str().into())
}

/// The diagnostics last published for `uri`.
fn published<'a>(messages: &'a [serde_json::Value], uri: &str) -> &'a [serde_json::Value] {
    notifications(messages, "textDocument/publishDiagnostics")
        .filter(|m| m["params"]["uri"] == uri)
        .last()
        .unwrap_or_else(|| panic!("Should have diagnosed {uri}: {messages:?}"))["params"]
        ["diagnostics"]
        .as_array()
        .expect("Diagnostics should be a list")
}

/// A workspace package calling what a `[dependencies]` entry declares in its
/// `.d.rn` sources - directly or through a dependency of its own - is not
/// reported, since the server compiles those declarations beside it.
#[test]
fn what_a_dependency_declares_is_not_reported() {
    let (mut s, uri) = dependency_session("app");
    s.open(&uri, "pub fn main() {\n    greet(base_value())\n}\n");

    let messages = s.run(|m| {
        notifications(m, "textDocument/publishDiagnostics").any(|m| m["params"]["uri"] == uri)
    });

    let diagnostics = published(&messages, &uri);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

/// What no dependency declares is still reported, so that the test above is
/// not passing because nothing is.
#[test]
fn what_no_dependency_declares_is_reported() {
    let (mut s, uri) = dependency_session("app");
    s.open(&uri, "pub fn main() {\n    greet(undeclared())\n}\n");

    let messages = s.run(|m| {
        notifications(m, "textDocument/publishDiagnostics").any(|m| m["params"]["uri"] == uri)
    });

    let diagnostics = published(&messages, &uri);

    assert!(
        diagnostics.iter().any(|d| d["message"]
            .as_str()
            .is_some_and(|m| m.contains("undeclared"))),
        "{diagnostics:?}"
    );
}

/// A dependency cycle is reported on the manifest which closes it, and does
/// not stop the server.
#[test]
fn a_dependency_cycle_is_reported_and_survived() {
    let (s, _) = dependency_session("cycle-a");

    let messages = s.run(|m| {
        notifications(m, "textDocument/publishDiagnostics").any(|m| {
            m["params"]["diagnostics"]
                .as_array()
                .is_some_and(|d| !d.is_empty())
        })
    });

    assert!(
        notifications(&messages, "textDocument/publishDiagnostics").any(|m| {
            m["params"]["uri"]
                .as_str()
                .is_some_and(|u| u.ends_with("cycle-b/Rune.toml"))
                && serde_json::to_string(&m["params"]["diagnostics"])
                    .is_ok_and(|d| d.contains("depends on"))
        }),
        "{messages:?}"
    );
}
