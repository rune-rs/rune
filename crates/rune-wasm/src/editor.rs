//! The playground editor.

use rune::ast::Spanned;
use rune::highlight::{highlight, Class};
use rune::syntax_tree;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlDocument, HtmlTextAreaElement, KeyboardEvent, MouseEvent, UrlSearchParams};
use yew::prelude::*;

use crate::listing::Row;
use crate::{compile_with, Config, WasmCompileResult, WasmDiagnosticKind};

const BUDGET: usize = 1_000_000;
const LINE_TRIM: usize = 80;
const LINES_TRIM: usize = 100;

#[derive(Debug, Default, Clone, Properties, PartialEq, Serialize, Deserialize)]
pub struct Props {
    pub source: String,
    /// Code appended to the source when it is run but not shown, from the
    /// lines of an embedded snippet which start with `# `.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prelude: String,
    /// Load a source shared through the page's `?c=` parameter, and offer to
    /// share the source as such a link.
    #[serde(default, skip_serializing_if = "is_false")]
    pub share: bool,
    /// Keep the source in local storage under this key as it is edited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_key: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub run_on_change: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// Extra JSON merged into the compile configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl Props {
    /// Split a snippet into what is shown and its hidden prelude.
    pub fn snippet(content: &str) -> (String, String) {
        let mut prelude = Vec::new();
        let mut rest = Vec::new();

        for line in content.split('\n') {
            // Rustdoc style: `#[attr]` at the start of a line is still code.
            match line
                .strip_prefix("# ")
                .or_else(|| (line == "#").then_some(""))
            {
                Some(line) => prelude.push(line),
                None => rest.push(line),
            }
        }

        let source = rest.join("\n").trim_matches('\n').to_owned();
        (source, prelude.join("\n"))
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

#[derive(Clone, Copy, PartialEq)]
pub struct Tip {
    line: usize,
    left: f64,
    top: f64,
}

/// What the pane beside the editor shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Instructions,
    SyntaxTree,
}

/// The syntax tree of the shown source, as rows for the side pane.
struct TreeRows {
    rows: Vec<TreeRow>,
    error: Option<(String, (usize, usize))>,
}

struct TreeRow {
    depth: usize,
    kind: String,
    span: (usize, usize),
    /// The source text of a token, as a literal.
    text: Option<String>,
}

/// Longest token text shown in the syntax tree, in characters.
const TREE_TEXT_TRIM: usize = 40;

impl TreeRows {
    fn parse(text: &str) -> Self {
        let tree = match syntax_tree::parse(text) {
            Ok(tree) => tree,
            Err(error) => {
                return Self {
                    rows: Vec::new(),
                    error: Some((error.to_string(), span_of(&error))),
                };
            }
        };

        let rows = tree
            .nodes
            .iter()
            .map(|node| {
                let span = (node.span.start.into_usize(), node.span.end.into_usize());

                let text = node
                    .is_token
                    .then(|| text.get(span.0..span.1))
                    .flatten()
                    .map(|text| {
                        let trimmed = text.chars().take(TREE_TEXT_TRIM).collect::<String>();

                        if trimmed.len() < text.len() {
                            format!("{trimmed:?}...")
                        } else {
                            format!("{trimmed:?}")
                        }
                    });

                // Only the variant's name, since what a token carries
                // (`Ident(Text(..))`) refers to the lexer's internals and the
                // token's text is shown beside it.
                let kind = format!("{:?}", node.kind);
                let kind = match kind.split_once('(') {
                    Some((name, _)) => name.to_owned(),
                    None => kind,
                };

                TreeRow {
                    depth: node.depth,
                    kind,
                    span,
                    text,
                }
            })
            .collect();

        let error = tree
            .error
            .as_ref()
            .map(|error| (error.to_string(), span_of(error)));

        Self { rows, error }
    }
}

fn span_of(error: &rune::compile::Error) -> (usize, usize) {
    let span = error.span();
    (span.start.into_usize(), span.end.into_usize())
}

pub enum Msg {
    Refresh,
    /// The textarea scrolled itself, which it must not since the editor
    /// around it is what scrolls.
    Unscroll,
    Tip(Option<Tip>),
    Share,
    Reset,
    /// The row of the side pane under the pointer.
    Hover(Option<usize>),
    Input,
    Key(KeyboardEvent),
    Caret,
    Run,
    /// Show or hide a side pane, which replaces whichever was shown.
    Side(Side, bool),
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
    editor: NodeRef,
    text: String,
    caret: usize,
    side: Option<Side>,
    /// Parsed while the syntax tree is shown, and again on every edit.
    tree: Option<TreeRows>,
    run_on_change: bool,
    running: bool,
    generation: u64,
    result: Option<Box<WasmCompileResult>>,
    marks: Vec<Mark>,
    hover: Option<usize>,
    /// The hovered code should be scrolled into view once it is rendered.
    reveal: bool,
    /// The caret should be scrolled into view once the edit is rendered.
    follow: bool,
    /// The gutter line whose diagnostics are shown, and where.
    tip: Option<Tip>,
    /// A link was just copied.
    shared: bool,
    /// The source has changed since the result was produced, so its spans no
    /// longer point at the right code.
    stale: bool,
}

impl Component for Editor {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let props = ctx.props();

        Self {
            textarea: NodeRef::default(),
            editor: NodeRef::default(),
            text: props.source.clone(),
            caret: 0,
            side: None,
            tree: None,
            run_on_change: props.run_on_change,
            running: false,
            generation: 0,
            result: None,
            marks: Vec::new(),
            hover: None,
            reveal: false,
            follow: false,
            tip: None,
            shared: false,
            stale: false,
        }
    }

    // Nothing here runs when the page is rendered at build time, so this is
    // where the browser-only parts start: the source from the URL, the textarea
    // and the first run.
    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if std::mem::take(&mut self.reveal) {
            self.reveal_hover();
        }

        if std::mem::take(&mut self.follow) {
            self.follow_caret();
        }

        if !first_render {
            return;
        }

        let props = ctx.props();
        let mut changed = false;

        // A shared link wins over what was stored, and is then taken out of
        // the address so that a reload does not bring it back over later edits.
        let shared = props.share.then(take_url_content).flatten();

        if let (Some(text), Some(key)) = (&shared, &props.storage_key) {
            storage::set(key, text);
        }

        let loaded = shared.or_else(|| props.storage_key.as_deref().and_then(storage::get));

        if let Some(text) = loaded.filter(|text| *text != self.text) {
            self.text = text;
            self.refresh_tree();
            changed = true;
        }

        // The textarea is uncontrolled so that the browser keeps its own undo
        // history, which assigning `value` would clear.
        if let Some(textarea) = self.textarea.cast::<HtmlTextAreaElement>() {
            textarea.set_value(&self.text);
        }

        if self.run_on_change {
            ctx.link().send_message(Msg::Run);
        } else if changed {
            ctx.link().send_message(Msg::Refresh);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Msg) -> bool {
        match msg {
            Msg::Refresh => true,
            Msg::Unscroll => {
                if let Some(textarea) = self.textarea.cast::<HtmlTextAreaElement>() {
                    textarea.set_scroll_top(0);
                    textarea.set_scroll_left(0);
                }

                false
            }
            Msg::Tip(tip) => {
                let changed = tip != self.tip;
                self.tip = tip;
                changed
            }
            Msg::Share => {
                if let Some(url) = share_url(&self.text) {
                    if let Some(window) = web_sys::window() {
                        let _ = window.navigator().clipboard().write_text(&url);
                    }

                    self.shared = true;
                }

                true
            }
            Msg::Reset => {
                let props = ctx.props();
                self.text = props.source.clone();

                if let Some(textarea) = self.textarea.cast::<HtmlTextAreaElement>() {
                    textarea.set_value(&self.text);
                }

                if let Some(key) = &props.storage_key {
                    storage::remove(key);
                }

                self.marks.clear();
                self.hover = None;
                self.stale = true;
                self.refresh_tree();
                self.run(ctx);
                true
            }
            Msg::Hover(hover) => {
                let changed = hover != self.hover;
                self.hover = hover;
                self.reveal = changed && hover.is_some();
                changed
            }
            Msg::Input => {
                let Some(textarea) = self.textarea.cast::<HtmlTextAreaElement>() else {
                    return false;
                };

                self.text = textarea.value();
                self.caret = caret(&textarea, &self.text);
                self.marks.clear();
                self.hover = None;
                self.stale = true;
                self.shared = false;
                self.follow = true;
                self.refresh_tree();

                if let Some(key) = &ctx.props().storage_key {
                    storage::set(key, &self.text);
                }

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
                self.follow = changed;
                changed
            }
            Msg::Run => {
                self.run(ctx);
                true
            }
            Msg::Side(side, value) => {
                self.hover = None;

                if value {
                    self.side = Some(side);
                } else if self.side == Some(side) {
                    self.side = None;
                }

                self.refresh_tree();

                let listed = self.result.as_ref().is_some_and(|r| !r.listing.is_empty());

                if self.side == Some(Side::Instructions) && !listed {
                    self.run(ctx);
                }

                true
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
                self.stale = false;

                if self.side == Some(Side::Instructions) {
                    self.hover = None;
                }

                self.marks = marks(&result, &self.text);
                self.result = Some(result);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let props = ctx.props();

        let side_title = match self.side {
            Some(Side::SyntaxTree) => "Syntax tree",
            _ => "Instructions",
        };

        let at_caret = self
            .marks
            .iter()
            .filter(|m| m.start <= self.caret && self.caret <= m.end)
            .map(|m| html! { <div class={classes!("rune-diagnostic", m.kind)}>{&m.message}</div> });

        html! {
            <>
            <div class={classes!("rune-workspace", self.side.is_some().then_some("with-side"))}>
            <div class="rune-editor" ref={self.editor.clone()}>
                <div class="rune-gutter" aria-hidden="true">{self.view_gutter(ctx)}</div>
                // The spare line keeps the textarea's text inside its box, so
                // that it never has anything to scroll.
                <pre class="rune-highlight" aria-hidden="true">{self.view_code()}{"\n "}</pre>
                <textarea
                    class="rune-input"
                    ref={self.textarea.clone()}
                    rows="1"
                    wrap="off"
                    spellcheck="false"
                    autocapitalize="off"
                    autocomplete="off"
                    oninput={link.callback(|_| Msg::Input)}
                    onkeydown={link.callback(Msg::Key)}
                    onkeyup={link.callback(|_| Msg::Caret)}
                    onclick={link.callback(|_| Msg::Caret)}
                    onselect={link.callback(|_| Msg::Caret)}
                    onscroll={link.callback(|_| Msg::Unscroll)}
                />
            </div>
            {self.view_tip()}
            // Always present and hidden instead, so that showing it does not
            // shift the editor among its siblings.
            <aside class="rune-side" aria-label={side_title} hidden={self.side.is_none()}>
                <div class="rune-side-head">
                    <span class="rune-side-title">{side_title}</span>
                    <span class="rune-side-hint">{"Hover one to see its code"}</span>
                </div>
                {self.view_tree_error()}
                <div class="rune-listing" onmouseleave={link.callback(|_| Msg::Hover(None))}>
                    {self.view_side(ctx)}
                </div>
            </aside>
            </div>
            // A stable wrapper, since yew matches unkeyed siblings from the end
            // and would otherwise rebuild the textarea when this changes.
            <div>{for at_caret}</div>
            <div class="rune-console">
                <div class="rune-control">
                    <button class="rune-button" disabled={self.running} onclick={link.callback(|_| Msg::Run)}>{"Run"}</button>
                    <label class="rune-option">
                        <input class="rune-checkbox" type="checkbox" title="Show compiled instructions"
                            checked={self.side == Some(Side::Instructions)}
                            onchange={link.callback(|e: Event| Msg::Side(Side::Instructions, checked(&e)))} />
                        {"Instructions"}
                    </label>
                    <label class="rune-option">
                        <input class="rune-checkbox" type="checkbox" title="Show the syntax tree the source parses into"
                            checked={self.side == Some(Side::SyntaxTree)}
                            onchange={link.callback(|e: Event| Msg::Side(Side::SyntaxTree, checked(&e)))} />
                        {"Syntax tree"}
                    </label>
                    <label class="rune-option">
                        <input class="rune-checkbox" type="checkbox" title="Run the code while typing"
                            checked={self.run_on_change}
                            onchange={link.callback(|e: Event| Msg::RunOnChange(checked(&e)))} />
                        {"Run on change"}
                    </label>
                    if props.share || props.storage_key.is_some() {
                        <span class="rune-control-end">
                            if props.storage_key.is_some() && self.text != props.source {
                                <button class="rune-button quiet" title="Go back to the example this started with"
                                    onclick={link.callback(|_| Msg::Reset)}>{"Reset"}</button>
                            }
                            if props.share {
                                <button class="rune-button quiet" title="Copy a link to this code"
                                    onclick={link.callback(|_| Msg::Share)}>
                                    {if self.shared { "Link copied" } else { "Share" }}
                                </button>
                            }
                        </span>
                    }
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
        let mut input = self.text.clone();

        if !props.prelude.is_empty() {
            input.push('\n');
            input.push_str(&props.prelude);
        }

        let config = props.config(self.side == Some(Side::Instructions));

        self.generation += 1;
        self.running = true;

        let generation = self.generation;
        let link = ctx.link().clone();

        wasm_bindgen_futures::spawn_local(async move {
            let result = compile_with(input, config).await;
            link.send_message(Msg::Done(generation, Box::new(result)));
        });
    }

    /// Scroll the editor, when it scrolls, so that the caret can be seen.
    /// The textarea is as large as its content and never scrolls itself, so
    /// the browser does not do this on its own.
    fn follow_caret(&self) {
        self.reveal(".rune-caret", 2.0);
    }

    /// Scroll the editor so that the code the hovered instruction came from
    /// can be seen.
    fn reveal_hover(&self) {
        self.reveal(".rune-hover", 3.0);
    }

    /// Scroll the editor until the element `selector` finds in the code is
    /// inside it, with `lines` of it as a margin above and below.
    fn reveal(&self, selector: &str, lines: f64) {
        let Some(editor) = self.editor.cast::<web_sys::Element>() else {
            return;
        };

        let Ok(Some(target)) = editor.query_selector(selector) else {
            return;
        };

        let gutter = editor
            .query_selector(".rune-gutter")
            .ok()
            .flatten()
            .map_or(0.0, |g| g.get_bounding_client_rect().width());

        let view = editor.get_bounding_client_rect();
        let target = target.get_bounding_client_rect();
        let line = target.height().max(16.0);
        let margin = line * lines;

        let top = view.top() + margin.min(view.height() / 3.0);
        let bottom = view.bottom() - margin.min(view.height() / 3.0);

        if target.top() < top {
            editor.set_scroll_top(editor.scroll_top() - (top - target.top()).ceil() as i32);
        } else if target.bottom() > bottom {
            editor.set_scroll_top(editor.scroll_top() + (target.bottom() - bottom).ceil() as i32);
        }

        let left = view.left() + gutter + line;
        let right = view.right() - line * 2.0;

        if target.left() < left {
            editor.set_scroll_left(editor.scroll_left() - (left - target.left()).ceil() as i32);
        } else if target.left() > right {
            editor.set_scroll_left(editor.scroll_left() + (target.left() - right).ceil() as i32);
        }
    }

    /// Line numbers, with the lines which have diagnostics marked by their
    /// worst kind.
    fn view_gutter(&self, ctx: &Context<Self>) -> Html {
        let lines = self.text.split('\n').count();
        let mut kinds = vec![None::<&'static str>; lines];

        for mark in &self.marks {
            let line = line_of(&self.text, mark.start).min(lines - 1);

            if kinds[line] != Some("error") {
                kinds[line] = Some(mark.kind);
            }
        }

        let rows = kinds.into_iter().enumerate().map(|(line, kind)| {
            let Some(kind) = kind else {
                return html! { <div class="rune-gutter-line">{line + 1}</div> };
            };

            let onmouseenter = ctx.link().callback(move |event: MouseEvent| {
                let rect = event
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .map(|e| e.get_bounding_client_rect());

                Msg::Tip(rect.map(|rect| Tip {
                    line,
                    left: rect.right(),
                    top: rect.top(),
                }))
            });

            html! {
                <div class={classes!("rune-gutter-line", "marked", kind)}
                    {onmouseenter} onmouseleave={ctx.link().callback(|_| Msg::Tip(None))}>
                    <span class={classes!("rune-gutter-marker", kind)}></span>
                    {line + 1}
                </div>
            }
        });

        html! { <>{for rows}</> }
    }

    /// The diagnostics of the hovered gutter line. It is fixed to the window
    /// rather than placed in the editor, which would clip it.
    fn view_tip(&self) -> Html {
        let messages = self.tip.map(|tip| {
            self.marks
                .iter()
                .filter(|m| line_of(&self.text, m.start) == tip.line)
                .map(|m| {
                    html! {
                        <p class={classes!("rune-tip-message", m.kind)}>{&m.message}</p>
                    }
                })
                .collect::<Html>()
        });

        let style = self
            .tip
            .map(|tip| format!("left: {}px; top: {}px;", tip.left + 8.0, tip.top - 4.0));

        html! {
            <div class="rune-tip" role="tooltip" hidden={self.tip.is_none()} {style}>
                {messages.unwrap_or_default()}
            </div>
        }
    }

    fn refresh_tree(&mut self) {
        self.tree = match self.side {
            Some(Side::SyntaxTree) => Some(TreeRows::parse(&self.text)),
            _ => None,
        };
    }

    /// The span of code the hovered row is for, within what is shown rather
    /// than the hidden prelude.
    fn hover_span(&self) -> Option<(usize, usize)> {
        if self.side == Some(Side::SyntaxTree) {
            let row = self.tree.as_ref()?.rows.get(self.hover?)?;
            let (start, end) = row.span;
            return (start < self.text.len()).then(|| (start, end.min(self.text.len())));
        }

        if self.stale {
            return None;
        }

        match self.result.as_ref()?.listing.get(self.hover?)? {
            Row::Inst {
                span: Some((start, end)),
                ..
            } if *start < self.text.len() => Some((*start, (*end).min(self.text.len()))),
            _ => None,
        }
    }

    fn view_side(&self, ctx: &Context<Self>) -> Html {
        match self.side {
            Some(Side::SyntaxTree) => self.view_tree(ctx),
            _ => self.view_listing(ctx),
        }
    }

    fn view_tree_error(&self) -> Html {
        if self.side != Some(Side::SyntaxTree) {
            return Html::default();
        }

        match self.tree.as_ref().and_then(|tree| tree.error.as_ref()) {
            Some((message, _)) => html! { <div class="rune-diagnostic error">{message}</div> },
            None => Html::default(),
        }
    }

    fn view_tree(&self, ctx: &Context<Self>) -> Html {
        let Some(tree) = &self.tree else {
            return Html::default();
        };

        let hovered = self
            .hover
            .and_then(|i| tree.rows.get(i))
            .map(|row| row.span);

        let rows = tree.rows.iter().enumerate().map(|(index, row)| {
            let current = self.hover == Some(index);
            let related = !current && hovered == Some(row.span);

            let indent = "  ".repeat(row.depth);
            let span = format!("@{}..{}", row.span.0, row.span.1);

            let class = classes!(
                "rune-listing-row",
                "node",
                current.then_some("hovered"),
                related.then_some("related"),
            );

            html! {
                <div {class} onmouseenter={ctx.link().callback(move |_| Msg::Hover(Some(index)))}>
                    {indent}
                    <span class={classes!("rune-node-kind", row.text.is_some().then_some("token"))}>{&row.kind}</span>
                    <span class="rune-node-span">{span}</span>
                    if let Some(text) = &row.text {
                        {" "}
                        <span class="rune-node-text">{text}</span>
                    }
                </div>
            }
        });

        html! { <>{for rows}</> }
    }

    fn view_listing(&self, ctx: &Context<Self>) -> Html {
        let Some(result) = &self.result else {
            return html! { <p class="rune-side-empty">{"Run the program to see what it compiles to."}</p> };
        };

        if result.listing.is_empty() {
            return html! { <p class="rune-side-empty">{"Nothing to show: the program did not compile."}</p> };
        }

        let hovered = match self.hover.and_then(|i| result.listing.get(i)) {
            Some(Row::Inst {
                span: Some(span), ..
            }) => Some(*span),
            _ => None,
        };

        let rows = result.listing.iter().enumerate().map(|(index, row)| match row {
            Row::Function { signature, hash } => html! {
                <div class="rune-listing-row function">
                    <span class="rune-inst-keyword">{"fn "}</span>
                    <span class="rune-inst-function">{signature}</span>
                    {" "}
                    <span class="rune-inst-hash">{hash}</span>
                </div>
            },
            Row::Label { name } => html! {
                <div class="rune-listing-row label">{name}{":"}</div>
            },
            Row::Inst {
                ip,
                op,
                args,
                comment,
                span,
            } => {
                let current = self.hover == Some(index);
                let related = !current && span.is_some() && *span == hovered;

                let args = args.iter().enumerate().map(|(n, arg)| {
                    html! {
                        <>
                        if n > 0 {
                            <span class="rune-inst-punctuation">{", "}</span>
                        }
                        if !arg.key.is_empty() {
                            <span class="rune-inst-key">{&arg.key}{"="}</span>
                        }
                        <span class={classes!("rune-inst-value", value_class(&arg.value))}>{&arg.value}</span>
                        </>
                    }
                });

                let class = classes!(
                    "rune-listing-row",
                    "inst",
                    current.then_some("hovered"),
                    related.then_some("related"),
                    span.is_none().then_some("unmapped"),
                    self.stale.then_some("stale"),
                );

                html! {
                    <div {class} onmouseenter={ctx.link().callback(move |_| Msg::Hover(Some(index)))}>
                        <span class="rune-inst-ip">{ip}</span>
                        <span class="rune-inst-op">{op}</span>
                        {" "}
                        {for args}
                        if let Some(comment) = comment {
                            <span class="rune-inst-comment">{" // "}{comment}</span>
                        }
                    </div>
                }
            }
        });

        html! { <>{for rows}</> }
    }

    fn view_code(&self) -> Html {
        let hover = self.hover_span();
        let text = self.text.as_str();
        let tokens = highlight(text).unwrap_or_default();
        let mut out = Vec::new();

        // An empty element at the caret, which is what following the caret
        // measures.
        let caret = self.caret.min(text.len());
        let mut placed = false;

        for token in tokens.iter() {
            let start = token.span.start.into_usize();
            let end = token.span.end.into_usize();

            let mut cuts = vec![start, end];

            let spans = self
                .marks
                .iter()
                .map(|m| (m.start, m.end))
                .chain(hover)
                .chain([(caret, caret)]);

            for (a, b) in spans {
                cuts.extend([a, b].into_iter().filter(|&c| start < c && c < end));
            }

            cuts.sort_unstable();
            cuts.dedup();

            for pair in cuts.windows(2) {
                let (a, b) = (pair[0], pair[1]);

                if a == caret && !placed {
                    out.push(html! { <span class="rune-caret"></span> });
                    placed = true;
                }

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

                let html = match hover {
                    Some((start, end)) if start <= a && b <= end => {
                        html! { <span class="rune-hover">{html}</span> }
                    }
                    _ => html,
                };

                out.push(html);
            }
        }

        if !placed {
            out.push(html! { <span class="rune-caret"></span> });
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

        html! { <>{primary}{diagnostics}</> }
    }
}

/// The zero-based line `offset` is on.
fn line_of(text: &str, offset: usize) -> usize {
    text.get(..offset)
        .map_or(0, |before| before.matches('\n').count())
}

fn value_class(value: &str) -> &'static str {
    let mut chars = value.chars();

    match (chars.next(), chars.next()) {
        (Some('0'), Some('x')) => "hash",
        (Some('"'), _) => "string",
        (Some(c), _) if c.is_ascii_digit() => "number",
        (Some('-'), Some(c)) if c.is_ascii_digit() => "number",
        _ => "plain",
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

/// The source in the address's `?c=` parameter, which is then removed.
fn take_url_content() -> Option<String> {
    let window = web_sys::window()?;
    let location = window.location();
    let params = UrlSearchParams::new_with_str(&location.search().ok()?).ok()?;
    let binary = window.atob(&params.get("c")?).ok()?;

    let bytes = binary
        .chars()
        .map(|c| u8::try_from(c as u32))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    params.delete("c");

    let rest = String::from(params.to_string());
    let path = location.pathname().ok()?;
    let url = if rest.is_empty() {
        path
    } else {
        format!("{path}?{rest}")
    };

    if let Ok(history) = window.history() {
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&url));
    }

    String::from_utf8(bytes).ok().filter(|s| !s.is_empty())
}

/// A link to this page which loads `content`.
fn share_url(content: &str) -> Option<String> {
    let window = web_sys::window()?;
    let location = window.location();
    let binary = content.bytes().map(char::from).collect::<String>();
    let encoded = window.btoa(&binary).ok()?;

    let params = UrlSearchParams::new().ok()?;
    params.set("c", &encoded);

    Some(format!(
        "{}{}?{}",
        location.origin().ok()?,
        location.pathname().ok()?,
        String::from(params.to_string())
    ))
}

/// Local storage, which may be missing or refuse writes, in which case the
/// source is simply not kept.
mod storage {
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    pub(super) fn get(key: &str) -> Option<String> {
        storage()?
            .get_item(key)
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
    }

    pub(super) fn set(key: &str, value: &str) {
        if let Some(storage) = storage() {
            let _ = storage.set_item(key, value);
        }
    }

    pub(super) fn remove(key: &str) {
        if let Some(storage) = storage() {
            let _ = storage.remove_item(key);
        }
    }
}
