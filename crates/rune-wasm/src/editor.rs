//! The playground editor, mounted on every `.rune` element of a page.

use rune::highlight::{highlight, Class};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlDocument, HtmlTextAreaElement, KeyboardEvent, UrlSearchParams};
use yew::prelude::*;

use crate::{compile_with, Config, WasmCompileResult, WasmDiagnosticKind};

const BUDGET: usize = 1_000_000;
const LINE_TRIM: usize = 80;
const LINES_TRIM: usize = 100;

#[wasm_bindgen(start)]
pub fn start() {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };

    let Ok(nodes) = document.query_selector_all(".rune") else {
        return;
    };

    for index in 0..nodes.length() {
        let Some(root) = nodes.item(index).and_then(|n| n.dyn_into::<Element>().ok()) else {
            continue;
        };

        if let Some(props) = Props::take(&document, &root) {
            let Ok(mount) = document.create_element("div") else {
                continue;
            };

            if root.prepend_with_node_1(&mount).is_ok() {
                yew::Renderer::<Editor>::with_root_and_props(mount, props).render();
            }
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct Props {
    source: String,
    prelude: String,
    update_url: bool,
    run_on_change: bool,
    options: Vec<String>,
    config: Option<String>,
}

impl Props {
    /// Read the settings off a `.rune` element and remove the markup the
    /// editor replaces.
    fn take(document: &web_sys::Document, root: &Element) -> Option<Self> {
        let editor = root.query_selector(".rune-editor").ok()??;
        let content = editor.text_content().unwrap_or_default();
        editor.remove();

        if let Ok(Some(console)) = root.query_selector(".rune-console") {
            console.remove();
        }

        let attr = |name: &str| root.get_attribute(name).filter(|v| !v.is_empty());
        let update_url = attr("rune-update-url").as_deref() == Some("true");

        let mut prelude = Vec::new();
        let mut rest = Vec::new();

        for line in content.split('\n') {
            match line.strip_prefix('#') {
                Some(line) => prelude.push(line),
                None => rest.push(line),
            }
        }

        let source = if update_url {
            url_content(document).unwrap_or_else(|| content.trim_start_matches('\n').to_owned())
        } else {
            rest.join("\n").trim_start_matches('\n').to_owned()
        };

        let options = attr("rune-options")
            .unwrap_or_default()
            .split(';')
            .map(str::trim)
            .filter(|o| !o.is_empty())
            .map(str::to_owned)
            .collect();

        Some(Self {
            source,
            prelude: prelude.join("\n"),
            update_url,
            run_on_change: attr("rune-run-on-change").as_deref() == Some("true"),
            options,
            config: attr("rune-config"),
        })
    }

    fn config(&self, instructions: bool) -> anyhow::Result<Config> {
        let mut config = serde_json::json!({
            "budget": BUDGET,
            "options": self.options,
            "instructions": instructions,
        });

        if let Some(extra) = &self.config {
            let extra: serde_json::Map<String, serde_json::Value> = serde_json::from_str(extra)?;

            if let Some(config) = config.as_object_mut() {
                config.extend(extra);
            }
        }

        Ok(serde_json::from_value(config)?)
    }
}

pub enum Msg {
    Input,
    Key(KeyboardEvent),
    Caret,
    Run,
    Instructions(bool),
    RunOnChange(bool),
    Done(u64, Box<WasmCompileResult>),
}

struct Mark {
    kind: &'static str,
    start: usize,
    end: usize,
    message: String,
}

pub struct Editor {
    textarea: NodeRef,
    text: String,
    caret: usize,
    instructions: bool,
    run_on_change: bool,
    running: bool,
    generation: u64,
    result: Option<Box<WasmCompileResult>>,
    marks: Vec<Mark>,
}

impl Component for Editor {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let props = ctx.props();

        if props.run_on_change {
            ctx.link().send_message(Msg::Run);
        }

        Self {
            textarea: NodeRef::default(),
            text: props.source.clone(),
            caret: 0,
            instructions: false,
            run_on_change: props.run_on_change,
            running: false,
            generation: 0,
            result: None,
            marks: Vec::new(),
        }
    }

    fn rendered(&mut self, _: &Context<Self>, first_render: bool) {
        // The textarea is uncontrolled so that the browser keeps its own undo
        // history, which assigning `value` would clear.
        if first_render {
            if let Some(textarea) = self.textarea.cast::<HtmlTextAreaElement>() {
                textarea.set_value(&self.text);
            }
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Msg) -> bool {
        match msg {
            Msg::Input => {
                let Some(textarea) = self.textarea.cast::<HtmlTextAreaElement>() else {
                    return false;
                };

                self.text = textarea.value();
                self.caret = caret(&textarea, &self.text);
                self.marks.clear();

                if self.run_on_change {
                    self.run(ctx);
                }

                true
            }
            Msg::Key(event) => {
                if event.key() == "Enter" && (event.ctrl_key() || event.meta_key()) {
                    event.prevent_default();
                    self.run(ctx);
                    return true;
                }

                if let Some(textarea) = self.textarea.cast::<HtmlTextAreaElement>() {
                    if keydown(&textarea, &event) {
                        event.prevent_default();
                    }
                }

                false
            }
            Msg::Caret => {
                let Some(textarea) = self.textarea.cast::<HtmlTextAreaElement>() else {
                    return false;
                };

                let caret = caret(&textarea, &self.text);
                let changed = caret != self.caret;
                self.caret = caret;
                changed && !self.marks.is_empty()
            }
            Msg::Run => {
                self.run(ctx);
                true
            }
            Msg::Instructions(value) => {
                self.instructions = value;
                false
            }
            Msg::RunOnChange(value) => {
                self.run_on_change = value;
                false
            }
            Msg::Done(generation, result) => {
                if generation != self.generation {
                    return false;
                }

                self.running = false;
                self.marks = marks(&result, &self.text);
                self.result = Some(result);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let at_caret = self
            .marks
            .iter()
            .filter(|m| m.start <= self.caret && self.caret <= m.end)
            .map(|m| html! { <div class={classes!("rune-diagnostic", m.kind)}>{&m.message}</div> });

        html! {
            <>
            <div class="rune-editor">
                <pre class="rune-highlight" aria-hidden="true">{self.view_code()}{"\n"}</pre>
                <textarea
                    class="rune-input"
                    ref={self.textarea.clone()}
                    rows="1"
                    spellcheck="false"
                    autocapitalize="off"
                    autocomplete="off"
                    oninput={link.callback(|_| Msg::Input)}
                    onkeydown={link.callback(Msg::Key)}
                    onkeyup={link.callback(|_| Msg::Caret)}
                    onclick={link.callback(|_| Msg::Caret)}
                />
            </div>
            // A stable wrapper, since yew matches unkeyed siblings from the end
            // and would otherwise rebuild the textarea when this changes.
            <div>{for at_caret}</div>
            <div class="rune-console">
                <div class="rune-control">
                    <button class="rune-button" disabled={self.running} onclick={link.callback(|_| Msg::Run)}>{"Run"}</button>
                    <label class="rune-option">
                        <input class="rune-checkbox" type="checkbox" title="Show compiled instructions"
                            checked={self.instructions}
                            onchange={link.callback(|e: Event| Msg::Instructions(checked(&e)))} />
                        {"Instructions"}
                    </label>
                    <label class="rune-option">
                        <input class="rune-checkbox" type="checkbox" title="Run the code while typing"
                            checked={self.run_on_change}
                            onchange={link.callback(|e: Event| Msg::RunOnChange(checked(&e)))} />
                        {"Run on change"}
                    </label>
                </div>
                {self.view_output()}
            </div>
            </>
        }
    }
}

impl Editor {
    fn run(&mut self, ctx: &Context<Self>) {
        let props = ctx.props();

        if props.update_url {
            set_url_content(&self.text);
        }

        let mut input = self.text.clone();

        if !props.prelude.is_empty() {
            input.push('\n');
            input.push_str(&props.prelude);
        }

        let config = props.config(self.instructions);

        self.generation += 1;
        self.running = true;

        let generation = self.generation;
        let link = ctx.link().clone();

        wasm_bindgen_futures::spawn_local(async move {
            let result = compile_with(input, config).await;
            link.send_message(Msg::Done(generation, Box::new(result)));
        });
    }

    fn view_code(&self) -> Html {
        let text = self.text.as_str();
        let tokens = highlight(text).unwrap_or_default();
        let mut out = Vec::new();

        for token in tokens.iter() {
            let start = token.span.start.into_usize();
            let end = token.span.end.into_usize();

            let mut cuts = vec![start, end];

            for m in &self.marks {
                cuts.extend(
                    [m.start, m.end]
                        .into_iter()
                        .filter(|&c| start < c && c < end),
                );
            }

            cuts.sort_unstable();
            cuts.dedup();

            for pair in cuts.windows(2) {
                let (a, b) = (pair[0], pair[1]);

                let Some(segment) = text.get(a..b) else {
                    continue;
                };

                let html = if token.class == Class::Whitespace {
                    html! { {segment} }
                } else {
                    html! { <span class={classes!("rune-token", token.class.as_str())}>{segment}</span> }
                };

                let html = match self.marks.iter().find(|m| m.start <= a && b <= m.end) {
                    Some(m) => html! { <span class={classes!("rune-mark", m.kind)}>{html}</span> },
                    None => html,
                };

                out.push(html);
            }
        }

        out.into_iter().collect()
    }

    fn view_output(&self) -> Html {
        if self.running && self.result.is_none() {
            return html! { <div class="rune-output">{"Running..."}</div> };
        }

        let Some(result) = &self.result else {
            return Html::default();
        };

        let mut sections = Vec::new();

        if let Some(output) = result.output.as_deref().filter(|s| !s.is_empty()) {
            sections.push(("Output", trim_output(output), false));
        }

        if let Some(error) = result.error.as_deref().filter(|s| !s.is_empty()) {
            sections.push(("Error", vec![error.to_owned()], true));
        }

        if let Some(value) = result.result.as_deref().filter(|s| !s.is_empty()) {
            sections.push(("Result", vec![value.to_owned()], false));
        }

        let primary = if self.running {
            html! { <div class="rune-output">{"Running..."}</div> }
        } else if sections.is_empty() {
            Html::default()
        } else {
            let sections = sections.into_iter().map(|(title, lines, error)| {
                let lines = lines.into_iter().map(|line| {
                    html! { <p class={classes!(error.then_some("rune-error"))}>{line}</p> }
                });

                html! { <><h4>{title}</h4>{for lines}</> }
            });

            html! { <div class="rune-output">{for sections}</div> }
        };

        let diagnostics = match result
            .diagnostics_output
            .as_deref()
            .filter(|s| !s.is_empty())
        {
            Some(text) => html! { <div class="rune-output diagnostics">{text}</div> },
            None => Html::default(),
        };

        let instructions = match result.instructions.as_deref().filter(|s| !s.is_empty()) {
            Some(text) => html! {
                <div class="rune-output instructions">{format!("# instruction\n{text}")}</div>
            },
            None => Html::default(),
        };

        html! { <>{primary}{diagnostics}{instructions}</> }
    }
}

/// Diagnostics as byte ranges into `text`, which the compiled source starts
/// with. An empty range is widened to a character so that it can be seen.
fn marks(result: &WasmCompileResult, text: &str) -> Vec<Mark> {
    let mut marks = Vec::new();

    for d in &result.diagnostics {
        let kind = match d.kind {
            WasmDiagnosticKind::Error => "error",
            WasmDiagnosticKind::Warning => "warning",
        };

        let mut start = floor(text, d.span.0.min(text.len()));
        let mut end = floor(text, d.span.1.min(text.len())).max(start);

        if start == end {
            if let Some(c) = text[end..].chars().next() {
                end += c.len_utf8();
            } else if let Some(c) = text[..start].chars().next_back() {
                start -= c.len_utf8();
            }
        }

        marks.push(Mark {
            kind,
            start,
            end,
            message: d.message.clone(),
        });
    }

    marks
}

fn floor(text: &str, mut at: usize) -> usize {
    while !text.is_char_boundary(at) {
        at -= 1;
    }

    at
}

fn trim_output(output: &str) -> Vec<String> {
    let mut lines = output
        .split('\n')
        .map(|line| {
            let count = line.chars().count();

            if count > LINE_TRIM {
                let head = line.chars().take(LINE_TRIM).collect::<String>();
                format!("{head} ... ({} trimmed)", count - LINE_TRIM)
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>();

    if lines.len() > LINES_TRIM {
        let more = lines.len() - LINES_TRIM;
        lines.truncate(LINES_TRIM);
        lines.push(format!("{more} more lines trimmed..."));
    }

    lines
}

fn checked(event: &Event) -> bool {
    event
        .target_dyn_into::<web_sys::HtmlInputElement>()
        .is_some_and(|input| input.checked())
}

/// The byte offset of the start of the selection.
fn caret(textarea: &HtmlTextAreaElement, text: &str) -> usize {
    let at = textarea.selection_start().ok().flatten().unwrap_or(0);
    byte_offset(text, at)
}

/// Convert a UTF-16 offset, which is what the DOM counts, into a byte offset.
fn byte_offset(text: &str, utf16: u32) -> usize {
    let mut count = 0;

    for (index, c) in text.char_indices() {
        if count >= utf16 {
            return index;
        }

        count += c.len_utf16() as u32;
    }

    text.len()
}

fn utf16(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

/// Edit through the browser's editing commands so that each edit lands on its
/// undo stack and fires `input` like typing does.
fn insert(textarea: &HtmlTextAreaElement, text: &str) {
    let document = textarea
        .owner_document()
        .and_then(|d| d.dyn_into::<HtmlDocument>().ok());

    let done = match document {
        Some(document) if text.is_empty() => document.exec_command("delete").unwrap_or(false),
        Some(document) => document
            .exec_command_with_show_ui_and_value("insertText", false, text)
            .unwrap_or(false),
        None => false,
    };

    if !done {
        let _ = textarea.set_range_text(text);
        let end = textarea.selection_end().ok().flatten().unwrap_or(0);
        let _ = textarea.set_selection_range(end + utf16(text), end + utf16(text));

        if let Ok(event) = web_sys::InputEvent::new("input") {
            let _ = textarea.dispatch_event(&event);
        }
    }
}

fn closer(c: char) -> Option<char> {
    match c {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        _ => None,
    }
}

/// Handle the keys which edit beyond inserting a character. Returns `true` if
/// the default action should be prevented.
fn keydown(textarea: &HtmlTextAreaElement, event: &KeyboardEvent) -> bool {
    if event.is_composing() || event.ctrl_key() || event.meta_key() || event.alt_key() {
        return false;
    }

    let value = textarea.value();
    let s16 = textarea.selection_start().ok().flatten().unwrap_or(0);
    let e16 = textarea.selection_end().ok().flatten().unwrap_or(s16);
    let start = byte_offset(&value, s16);
    let end = byte_offset(&value, e16);
    let before = value[..start].chars().next_back();
    let after = value[end..].chars().next();
    let line_start = value[..start].rfind('\n').map_or(0, |i| i + 1);

    let select = |a: u32, b: u32| {
        let _ = textarea.set_selection_range(a, b);
    };

    match event.key().as_str() {
        "Tab" => {
            let multiline = value[start..end].contains('\n');

            if !event.shift_key() && !multiline {
                insert(textarea, "\t");
                return true;
            }

            let block_end = if end > start && value[..end].ends_with('\n') {
                end - 1
            } else {
                value[end..].find('\n').map_or(value.len(), |i| end + i)
            };

            let block = &value[line_start..block_end];
            let mut removed = 0;

            let lines = block
                .split('\n')
                .enumerate()
                .map(|(n, line)| {
                    if !event.shift_key() {
                        return format!("\t{line}");
                    }

                    let strip = if line.starts_with('\t') {
                        1
                    } else {
                        (line.len() - line.trim_start_matches(' ').len()).min(4)
                    };

                    if n == 0 {
                        removed = strip;
                    }

                    line[strip..].to_owned()
                })
                .collect::<Vec<_>>()
                .join("\n");

            if lines == block {
                return true;
            }

            let ls16 = utf16(&value[..line_start]);
            select(ls16, utf16(&value[..block_end]));
            insert(textarea, &lines);

            if multiline {
                select(ls16, ls16 + utf16(&lines));
            } else {
                let caret = s16.saturating_sub(removed as u32).max(ls16);
                select(caret, caret);
            }

            true
        }
        "Enter" if !event.shift_key() => {
            let line = &value[line_start..start];
            let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
            let open = value[..start]
                .trim_end_matches([' ', '\t'])
                .chars()
                .next_back();
            let mut text = format!("\n{indent}");

            match open.and_then(closer) {
                Some(close) => {
                    text.push_str(if indent.ends_with(' ') { "    " } else { "\t" });
                    let caret = s16 + utf16(&text);

                    if after == Some(close) {
                        text.push('\n');
                        text.push_str(indent);
                    }

                    insert(textarea, &text);
                    select(caret, caret);
                }
                None => insert(textarea, &text),
            }

            true
        }
        key @ ("(" | "[" | "{") if start == end => {
            let open = key.chars().next().unwrap_or_default();

            let pairs = match after {
                None => true,
                Some(c) => c.is_whitespace() || matches!(c, ')' | ']' | '}' | ',' | ';'),
            };

            if !pairs {
                return false;
            }

            if let Some(close) = closer(open) {
                insert(textarea, &format!("{open}{close}"));
                select(s16 + 1, s16 + 1);
            }

            true
        }
        key @ (")" | "]" | "}" | "\"")
            if start == end && after.map(String::from).as_deref() == Some(key) =>
        {
            select(s16 + 1, s16 + 1);
            true
        }
        "\"" if start == end => {
            let pairs = after
                .is_none_or(|c| c.is_whitespace() || matches!(c, ')' | ']' | '}' | ',' | ';'))
                && before.is_none_or(|c| !c.is_alphanumeric() && c != '\\' && c != '"');

            if !pairs {
                return false;
            }

            insert(textarea, "\"\"");
            select(s16 + 1, s16 + 1);
            true
        }
        "Backspace" if start == end => {
            let paired = match (before, after) {
                (Some('"'), Some('"')) => true,
                (Some(open), Some(close)) => closer(open) == Some(close),
                _ => false,
            };

            if !paired {
                return false;
            }

            select(s16 - 1, s16 + 1);
            insert(textarea, "");
            true
        }
        _ => false,
    }
}

fn url_content(document: &web_sys::Document) -> Option<String> {
    let window = web_sys::window()?;
    let search = document.location()?.search().ok()?;
    let params = UrlSearchParams::new_with_str(&search).ok()?;
    let binary = window.atob(&params.get("c")?).ok()?;

    let bytes = binary
        .chars()
        .map(|c| u8::try_from(c as u32))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    String::from_utf8(bytes).ok().filter(|s| !s.is_empty())
}

fn set_url_content(content: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let Ok(search) = window.location().search() else {
        return;
    };

    let Ok(params) = UrlSearchParams::new_with_str(&search) else {
        return;
    };

    let binary = content.bytes().map(char::from).collect::<String>();

    let Ok(encoded) = window.btoa(&binary) else {
        return;
    };

    params.set("c", &encoded);

    if let Ok(history) = window.history() {
        let url = format!("?{}", String::from(params.to_string()));
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&url));
    }
}
