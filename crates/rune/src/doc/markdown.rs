use core::fmt;

use crate::alloc::fmt::TryWrite;
use crate::alloc::{self, try_vec, HashMap, String, Vec};
use crate::doc::TestParams;

use anyhow::Result;
use pulldown_cmark::{Alignment, CodeBlockKind, CowStr, Event, LinkType, Tag, TagEnd};
use pulldown_cmark_escape::{escape_href, escape_html, StrWrite};
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxReference, SyntaxSet};

pub(crate) const RUST_TOKEN: &str = "rust";
pub(crate) const RUNE_TOKEN: &str = "rune";

use Event::*;

enum TableState {
    Head,
    Body,
}

struct StringWriter<'a> {
    string: &'a mut String,
}

impl StrWrite for StringWriter<'_> {
    type Error = alloc::Error;

    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
        self.string.try_push_str(s)
    }

    fn write_fmt(&mut self, args: fmt::Arguments) -> Result<(), Self::Error> {
        TryWrite::write_fmt(self.string, args)
    }
}

struct Writer<'a, 'o, I> {
    syntax_set: Option<&'a SyntaxSet>,
    iter: I,
    out: StringWriter<'o>,
    tests: Option<&'o mut Vec<(String, TestParams)>>,
    codeblock: Option<(
        Option<(&'a SyntaxSet, &'a SyntaxReference)>,
        Option<TestParams>,
    )>,
    table_state: TableState,
    table_alignments: Vec<Alignment>,
    table_cell_index: usize,
    numbers: HashMap<CowStr<'a>, usize>,
}

impl<'a, I> Writer<'a, '_, I>
where
    I: Iterator<Item = Event<'a>>,
{
    #[inline]
    fn write(&mut self, s: &str) -> Result<()> {
        self.out.string.try_push_str(s)?;
        Ok(())
    }

    fn run(mut self) -> Result<()> {
        while let Some(event) = self.iter.next() {
            match event {
                Start(tag) => {
                    self.start_tag(tag)?;
                }
                End(tag) => {
                    self.end_tag(tag)?;
                }
                Text(text) => {
                    if let Some((syntax, params)) = self.codeblock {
                        let mut string = String::new();

                        let s = (self.tests.is_some() && params.is_some()).then_some(&mut string);

                        let html = match syntax {
                            Some((syntax_set, syntax)) => {
                                render_code_by_syntax(syntax_set, syntax, text.lines(), s)?
                            }
                            None => render_code_without_syntax(text.lines(), s)?,
                        };

                        if let Some(params) = params {
                            if let Some(tests) = self.tests.as_mut() {
                                tests.try_push((string, params))?;
                            }
                        }

                        self.write(&html)?;
                    } else {
                        escape_html(&mut self.out, &text)?;
                    }
                }
                Code(text) => {
                    self.write("<code>")?;
                    escape_html(&mut self.out, &text)?;
                    self.write("</code>")?;
                }
                Html(html) => {
                    self.write(&html)?;
                }
                SoftBreak => {
                    self.write(" ")?;
                }
                HardBreak => {
                    self.write("<br />")?;
                }
                Rule => {
                    self.write("<hr />")?;
                }
                FootnoteReference(name) => {
                    let len = self.numbers.len() + 1;
                    self.write("<sup class=\"footnote-reference\"><a href=\"#")?;
                    escape_html(&mut self.out, &name)?;
                    self.write("\">")?;
                    let number = *self.numbers.entry(name).or_try_insert(len)?;
                    write!(&mut self.out, "{number}")?;
                    self.write("</a></sup>")?;
                }
                TaskListMarker(true) => {
                    self.write("<input disabled=\"\" type=\"checkbox\" checked=\"\"/>")?;
                }
                TaskListMarker(false) => {
                    self.write("<input disabled=\"\" type=\"checkbox\"/>")?;
                }
                InlineMath(text) => {
                    self.write(r#"<span class="math math-inline">"#)?;
                    escape_html(&mut self.out, &text)?;
                    self.write("</span>")?;
                }
                DisplayMath(text) => {
                    self.write(r#"<span class="math math-display">"#)?;
                    escape_html(&mut self.out, &text)?;
                    self.write("</span>")?;
                }
                InlineHtml(text) => {
                    self.write(&text)?;
                }
            }
        }

        Ok(())
    }

    fn start_tag(&mut self, tag: Tag<'a>) -> Result<()> {
        match tag {
            Tag::Paragraph => {
                self.write("<p>")?;
            }
            Tag::Heading {
                level, id, classes, ..
            } => {
                self.write("<")?;

                write!(&mut self.out, "{level}")?;

                if let Some(id) = id {
                    self.write(" id=\"")?;
                    escape_html(&mut self.out, &id)?;
                    self.write("\"")?;
                }

                let mut classes = classes.iter();

                if let Some(class) = classes.next() {
                    self.write(" class=\"")?;
                    escape_html(&mut self.out, class)?;
                    for class in classes {
                        self.write(" ")?;
                        escape_html(&mut self.out, class)?;
                    }
                    self.write("\"")?;
                }

                self.write(">")?;
            }
            Tag::Table(alignments) => {
                self.table_alignments = alignments.try_into()?;
                self.write("<table>")?;
            }
            Tag::TableHead => {
                self.table_state = TableState::Head;
                self.table_cell_index = 0;
                self.write("<thead><tr>")?;
            }
            Tag::TableRow => {
                self.table_cell_index = 0;
                self.write("<tr>")?;
            }
            Tag::TableCell => {
                match self.table_state {
                    TableState::Head => {
                        self.write("<th")?;
                    }
                    TableState::Body => {
                        self.write("<td")?;
                    }
                }

                match self.table_alignments.get(self.table_cell_index) {
                    Some(Alignment::Left) => {
                        self.write(" style=\"text-align: left\">")?;
                    }
                    Some(Alignment::Center) => {
                        self.write(" style=\"text-align: center\">")?;
                    }
                    Some(Alignment::Right) => {
                        self.write(" style=\"text-align: right\">")?;
                    }
                    _ => {
                        self.write(">")?;
                    }
                }
            }
            Tag::BlockQuote(..) => {
                self.write("<blockquote>")?;
            }
            Tag::CodeBlock(kind) => {
                self.write("<pre><code class=\"language-")?;
                let (lang, syntax, params) = self.find_syntax(&kind);
                self.codeblock = Some((syntax, params));
                escape_href(&mut self.out, lang)?;
                self.write("\">")?;
            }
            Tag::List(Some(1)) => {
                self.write("<ol>")?;
            }
            Tag::List(Some(start)) => {
                self.write("<ol start=\"")?;
                write!(&mut self.out, "{start}")?;
                self.write("\">")?;
            }
            Tag::List(None) => {
                self.write("<ul>")?;
            }
            Tag::Item => {
                self.write("<li>")?;
            }
            Tag::Emphasis => {
                self.write("<em>")?;
            }
            Tag::Strong => {
                self.write("<strong>")?;
            }
            Tag::Strikethrough => {
                self.write("<del>")?;
            }
            Tag::Link {
                link_type: LinkType::Email,
                dest_url,
                title,
                ..
            } => {
                self.write("<a href=\"mailto:")?;
                escape_href(&mut self.out, &dest_url)?;
                if !title.is_empty() {
                    self.write("\" title=\"")?;
                    escape_html(&mut self.out, &title)?;
                }
                self.write("\">")?;
            }
            Tag::Link {
                dest_url, title, ..
            } => {
                self.write("<a href=\"")?;
                escape_href(&mut self.out, &dest_url)?;
                if !title.is_empty() {
                    self.write("\" title=\"")?;
                    escape_html(&mut self.out, &title)?;
                }
                self.write("\">")?;
            }
            Tag::Image {
                dest_url, title, ..
            } => {
                self.write("<img src=\"")?;
                escape_href(&mut self.out, &dest_url)?;
                self.write("\" alt=\"")?;
                self.raw_text()?;

                if !title.is_empty() {
                    self.write("\" title=\"")?;
                    escape_html(&mut self.out, &title)?;
                }

                self.write("\" />")?;
            }
            Tag::FootnoteDefinition(name) => {
                self.write("<div class=\"footnote-definition\" id=\"")?;
                escape_html(&mut self.out, &name).map_err(|_| fmt::Error)?;
                self.write("\"><sup class=\"footnote-definition-label\">")?;
                let len = self.numbers.len() + 1;
                let number = *self.numbers.entry(name).or_try_insert(len)?;
                write!(&mut self.out, "{number}")?;
                self.write("</sup>")?;
            }
            Tag::HtmlBlock => {}
            Tag::DefinitionList => {
                self.write("<dl>")?;
            }
            Tag::DefinitionListTitle => {
                self.write("<dt>")?;
            }
            Tag::DefinitionListDefinition => {
                self.write("<dd>")?;
            }
            Tag::MetadataBlock(..) => {}
            Tag::Superscript => {
                self.write("<sup>")?;
            }
            Tag::Subscript => {
                self.write("<sub>")?;
            }
        }

        Ok(())
    }

    fn find_syntax<'input>(
        &mut self,
        kind: &'input CodeBlockKind<'input>,
    ) -> (
        &'input str,
        Option<(&'a SyntaxSet, &'a SyntaxReference)>,
        Option<TestParams>,
    ) {
        let mut syntax = None;
        let mut params = TestParams::default();

        if let CodeBlockKind::Fenced(fences) = &kind {
            for token in fences.split(',') {
                let (token, lookup, is_rune) = match token.trim() {
                    "no_run" => {
                        params.no_run = true;
                        continue;
                    }
                    "should_panic" => {
                        params.should_panic = true;
                        continue;
                    }
                    "ignore" => {
                        params.ignore = true;
                        continue;
                    }
                    RUNE_TOKEN => (RUNE_TOKEN, RUST_TOKEN, true),
                    token => (token, token, false),
                };

                if syntax.is_none() {
                    match self.syntax_set {
                        Some(syntax_set) => {
                            if let Some(s) = syntax_set.find_syntax_by_token(lookup) {
                                syntax = Some((token, Some((syntax_set, s)), is_rune));
                            }
                        }
                        None => {
                            syntax = Some((token, None, is_rune));
                        }
                    }
                }
            }
        }

        if let Some((token, syntax, is_rune)) = syntax {
            return (token, syntax, is_rune.then_some(params));
        }

        if let Some(syntax_set) = self.syntax_set {
            let Some(syntax) = syntax_set.find_syntax_by_token(RUST_TOKEN) else {
                return (
                    "text",
                    Some((syntax_set, syntax_set.find_syntax_plain_text())),
                    Some(params),
                );
            };

            (RUNE_TOKEN, Some((syntax_set, syntax)), Some(params))
        } else {
            (RUNE_TOKEN, None, Some(params))
        }
    }

    fn end_tag(&mut self, tag: TagEnd) -> Result<()> {
        match tag {
            TagEnd::Paragraph => {
                self.write("</p>")?;
            }
            TagEnd::Heading(level) => {
                self.write("</")?;
                write!(&mut self.out, "{level}")?;
                self.write(">")?;
            }
            TagEnd::Table => {
                self.write("</tbody></table>")?;
            }
            TagEnd::TableHead => {
                self.write("</tr></thead><tbody>")?;
                self.table_state = TableState::Body;
            }
            TagEnd::TableRow => {
                self.write("</tr>")?;
            }
            TagEnd::TableCell => {
                match self.table_state {
                    TableState::Head => {
                        self.write("</th>")?;
                    }
                    TableState::Body => {
                        self.write("</td>")?;
                    }
                }
                self.table_cell_index += 1;
            }
            TagEnd::BlockQuote(_) => {
                self.write("</blockquote>")?;
            }
            TagEnd::CodeBlock => {
                self.write("</code></pre>")?;
                self.codeblock = None;
            }
            TagEnd::List(true) => {
                self.write("</ol>")?;
            }
            TagEnd::List(false) => {
                self.write("</ul>")?;
            }
            TagEnd::Item => {
                self.write("</li>")?;
            }
            TagEnd::Emphasis => {
                self.write("</em>")?;
            }
            TagEnd::Strong => {
                self.write("</strong>")?;
            }
            TagEnd::Strikethrough => {
                self.write("</del>")?;
            }
            TagEnd::Link => {
                self.write("</a>")?;
            }
            TagEnd::Image => (),
            TagEnd::FootnoteDefinition => {
                self.write("</div>")?;
            }
            TagEnd::HtmlBlock => {}
            TagEnd::DefinitionList => {
                self.write("</dl>")?;
            }
            TagEnd::DefinitionListTitle => {
                self.write("</dt>")?;
            }
            TagEnd::DefinitionListDefinition => {
                self.write("</dd>")?;
            }
            TagEnd::MetadataBlock(..) => {}
            TagEnd::Superscript => {
                self.write("</sup>")?;
            }
            TagEnd::Subscript => {
                self.write("</sub>")?;
            }
        }

        Ok(())
    }

    fn raw_text(&mut self) -> Result<()> {
        let mut nest = 0;

        while let Some(event) = self.iter.next() {
            match event {
                Start(_) => nest += 1,
                End(_) => {
                    if nest == 0 {
                        break;
                    }
                    nest -= 1;
                }
                Html(text) | Code(text) | Text(text) | InlineMath(text) | DisplayMath(text)
                | InlineHtml(text) => {
                    escape_html(&mut self.out, &text).map_err(|_| fmt::Error)?;
                }
                SoftBreak | HardBreak | Rule => {
                    self.write(" ")?;
                }
                FootnoteReference(name) => {
                    let len = self.numbers.len() + 1;
                    let number = *self.numbers.entry(name).or_try_insert(len)?;
                    write!(self.out, "[{number}]")?;
                }
                TaskListMarker(true) => self.write("[x]")?,
                TaskListMarker(false) => self.write("[ ]")?,
            }
        }

        Ok(())
    }
}

/// Process markdown html and captures tests.
pub(crate) fn push_html<'a, I>(
    syntax_set: Option<&'a SyntaxSet>,
    string: &'a mut String,
    iter: I,
    tests: Option<&'a mut Vec<(String, TestParams)>>,
) -> Result<()>
where
    I: Iterator<Item = Event<'a>>,
{
    let writer = Writer {
        syntax_set,
        iter,
        out: StringWriter { string },
        tests,
        codeblock: None,
        table_state: TableState::Head,
        table_alignments: try_vec![],
        table_cell_index: 0,
        numbers: HashMap::new(),
    };

    writer.run()?;
    Ok(())
}

/// Render documentation.
pub(super) fn render_code_by_syntax(
    syntax_set: &SyntaxSet,
    syntax: &SyntaxReference,
    lines: impl IntoIterator<Item: AsRef<str>>,
    mut out: Option<&mut String>,
) -> Result<String> {
    let mut buf = String::new();
    let mut gen =
        ClassedHTMLGenerator::new_with_class_style(syntax, syntax_set, ClassStyle::Spaced);

    for line in lines {
        let line = line.as_ref();
        let line = line.strip_prefix(' ').unwrap_or(line);

        if line.starts_with('#') {
            if let Some(o) = out.as_mut() {
                o.try_push_str(line.trim_start_matches('#'))?;
                o.try_push('\n')?;
            }

            continue;
        }

        if let Some(o) = out.as_mut() {
            o.try_push_str(line)?;
            o.try_push('\n')?;
        }

        buf.clear();
        buf.try_push_str(line)?;
        buf.try_push('\n')?;
        gen.parse_html_for_line_which_includes_newline(&buf)?;
    }

    Ok(gen.finalize().try_into()?)
}

pub(super) fn render_code_without_syntax(
    lines: impl IntoIterator<Item: AsRef<str>>,
    mut out: Option<&mut String>,
) -> Result<String> {
    let mut buf = String::new();

    for line in lines {
        let line = line.as_ref();
        let line = line.strip_prefix(' ').unwrap_or(line);

        if line.starts_with('#') {
            if let Some(o) = out.as_mut() {
                o.try_push_str(line.trim_start_matches('#'))?;
                o.try_push('\n')?;
            }

            continue;
        }

        if let Some(o) = out.as_mut() {
            o.try_push_str(line)?;
            o.try_push('\n')?;
        }

        buf.try_push_str(line)?;
        buf.try_push('\n')?;
    }

    Ok(buf)
}
