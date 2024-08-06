use core::mem::take;

use crate::alloc::prelude::*;
use crate::alloc::{self, VecDeque};
use crate::ast::{Kind, Span};
use crate::compile::{Error, ErrorKind, FmtOptions, Result, WithSpan};
use crate::grammar::{Node, Tree};

use super::{INDENT, NL, NL_CHAR, WS};

/// Hint for how comments may be laid out.
pub(super) enum Comments {
    /// Any kind of comment can be inserted and should be line-separated.
    Line,
    /// Comments may be inserted in a whitespace prefix position, like `(<comment>args`.
    Prefix,
    /// Comments may be inserted in a whitespace suffix position, like `args<comment>)`.
    Suffix,
    /// An infix comment hint, like `(<comment>)` where there is no preceeding
    /// or succeeding whitespace.
    Infix,
}

#[derive(Clone, Copy, Debug)]
struct Comment {
    span: Span,
    before: usize,
    line: bool,
}

/// A source of text.
#[repr(transparent)]
pub(super) struct Source(str);

impl Source {
    fn new(source: &str) -> &Self {
        // Safety: Source is repr transparent over str.
        unsafe { &*(source as *const str as *const Self) }
    }

    /// Get a checked span from the source.
    pub(super) fn get(&self, span: Span) -> Result<&str> {
        let Some(source) = self.0.get(span.range()) else {
            return Err(Error::new(span, ErrorKind::BadSpan { len: self.0.len() }));
        };

        Ok(source)
    }

    /// Perform a whitespace-insensitive count and check if it's more than
    /// `count`.
    pub(super) fn is_at_least(&self, span: Span, mut count: usize) -> Result<bool> {
        let source = self.get(span)?;

        for c in source.chars() {
            if c.is_whitespace() {
                continue;
            }

            let Some(c) = count.checked_sub(1) else {
                return Ok(true);
            };

            count = c;
        }

        Ok(false)
    }
}

/// The output buffer.
#[repr(transparent)]
pub(super) struct Buffer(String);

impl Buffer {
    fn new(output: &mut String) -> &mut Self {
        // Safety: Source is repr transparent over str.
        unsafe { &mut *(output as *mut String as *mut Self) }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    fn str(&mut self, s: &str) -> alloc::Result<()> {
        self.0.try_push_str(s)
    }

    fn lines(&mut self, indent: usize, lines: usize) -> alloc::Result<()> {
        if lines == 0 {
            return Ok(());
        }

        for _ in 0..lines {
            self.0.try_push_str(NL)?;
        }

        for _ in 0..indent {
            self.0.try_push_str(INDENT)?;
        }

        Ok(())
    }
}

/// A constructed syntax tree.
pub(crate) struct Output<'a> {
    span: Span,
    pub(super) source: &'a Source,
    o: &'a mut Buffer,
    pub(super) options: &'a FmtOptions,
    comments: VecDeque<Comment>,
    lines: usize,
    use_lines: bool,
    ws: bool,
    indent: usize,
}

impl<'a> Output<'a> {
    /// Construct a new tree.
    pub(super) fn new(
        span: Span,
        source: &'a str,
        o: &'a mut String,
        options: &'a FmtOptions,
    ) -> Self {
        Self {
            span,
            source: Source::new(source),
            o: Buffer::new(o),
            options,
            comments: VecDeque::new(),
            lines: 0,
            use_lines: false,
            ws: false,
            indent: 0,
        }
    }

    /// Ignore the given node.
    pub(crate) fn ignore(&mut self, node: Node<'a>) -> Result<()> {
        self.process_comments(node.walk_from())?;
        Ok(())
    }

    /// Write the give node to output.
    pub(crate) fn write(&mut self, node: Node<'a>) -> Result<()> {
        self.flush_whitespace(false)?;
        self.write_node(&node)?;
        self.process_comments(node.walk_from())?;
        Ok(())
    }

    /// Write the give node to output without comment or whitespace processing.
    pub(crate) fn write_raw(&mut self, node: Node<'a>) -> Result<()> {
        self.write_node(&node)?;
        Ok(())
    }

    /// Buffer literal to output.
    pub(crate) fn lit(&mut self, s: &str) -> Result<()> {
        // We want whitespace to be preserved *unless* it was written out, since
        // a literal is a synthetic token.
        self.flush_whitespace(true)?;
        self.o.str(s).with_span(self.span)?;
        Ok(())
    }

    /// Flush any remaining whitespace.
    pub(super) fn comments(&mut self, comments: Comments) -> Result<()> {
        if self.comments.is_empty() {
            return Ok(());
        }

        match comments {
            Comments::Line => {
                self.comments_line(false)?;
            }
            Comments::Prefix | Comments::Suffix => {
                // Confusingly, the comment hint determines the location of the
                // comment relative to any relevant token, so it *looks* like
                // they are flipped here. But the writer function is simply used
                // to determine where the whitespace should be located.
                self.comments_ws(
                    matches!(comments, Comments::Suffix),
                    matches!(comments, Comments::Prefix),
                )?;
            }
            Comments::Infix => {
                self.comments_ws(false, false)?;
            }
        }

        Ok(())
    }

    /// Indent the output.
    pub(super) fn indent(&mut self, indent: isize) -> Result<()> {
        if indent != 0 {
            self.indent = self.checked_indent(indent)?;
        }

        Ok(())
    }

    /// Emit a line hint, indicating that the next write should be on a new line
    /// separated by at least `nl` lines.
    ///
    /// The value of `nl` is clamped to the range `[0, 2]`.
    ///
    /// This will write any pending line comments which are on the same line as
    /// the previously written nodes.
    pub(crate) fn nl(&mut self, lines: usize) -> Result<()> {
        if lines == 0 {
            return Ok(());
        }

        self.comments_line(true)?;

        // If we don't already have line heuristics, adopt the proposed one.
        if self.lines == 0 {
            self.lines = lines;
        }

        // At this point, we will use lines for the next flush.
        self.use_lines = true;
        Ok(())
    }

    /// Emit a whitespace hint, indicating that the next node write should
    /// happen with preceeding whitespace.
    ///
    /// This emits a `Comments::Suffix` hint by default, since we *expect*
    /// whitespace to be followed by tokens which will add any additional
    /// whitespace.
    pub(super) fn ws(&mut self) -> Result<()> {
        self.comments_ws(true, false)?;
        self.ws = true;
        Ok(())
    }

    /// Write leading comments.
    pub(super) fn flush_prefix_comments(&mut self, tree: &'a Tree) -> Result<()> {
        self.process_comments(tree.walk())?;
        self.comments(Comments::Line)?;
        self.use_lines = self.lines > 0;
        Ok(())
    }

    /// Smuggle in line comments when we receive a line hint.
    fn comments_line(&mut self, same_line: bool) -> Result<()> {
        while let Some(c) = self.comments.front() {
            if same_line && c.before != 0 {
                break;
            }

            if !self.o.is_empty() {
                if c.before == 0 {
                    self.o.str(WS).with_span(c.span)?;
                } else {
                    self.o
                        .lines(self.indent, c.before.min(2))
                        .with_span(c.span)?;
                }
            }

            let source = self.source.get(c.span)?;
            let source = if c.line { source.trim_end() } else { source };
            self.o.str(source).with_span(c.span)?;

            _ = self.comments.pop_front();
        }

        Ok(())
    }

    /// Smuggle in whitespace comments when we receive a whitespace hint.
    fn comments_ws(&mut self, prefix: bool, suffix: bool) -> Result<()> {
        if self.comments.is_empty() {
            return Ok(());
        }

        let mut any = false;

        while let Some(c) = self.comments.front() {
            if c.line {
                break;
            }

            if (prefix || any) && !self.o.is_empty() {
                self.o.str(WS).with_span(c.span)?;
            }

            let source = self.source.get(c.span)?;
            self.o.str(source).with_span(c.span)?;

            any = true;

            _ = self.comments.pop_front();
        }

        if suffix && any {
            self.o.str(WS).with_span(self.span)?;
        }

        Ok(())
    }

    fn process_comments<I>(&mut self, iter: I) -> Result<()>
    where
        I: IntoIterator<Item = Node<'a>>,
    {
        for node in iter {
            if !node.has_children() && !self.write_comment(node)? {
                break;
            }
        }

        Ok(())
    }

    fn write_comment(&mut self, node: Node<'a>) -> Result<bool> {
        let span = node.span();

        match node.kind() {
            Kind::Comment | Kind::MultilineComment(..) => {
                self.comments
                    .try_push_back(Comment {
                        span,
                        before: take(&mut self.lines),
                        line: matches!(node.kind(), Kind::Comment),
                    })
                    .with_span(span)?;

                Ok(true)
            }
            Kind::Whitespace => {
                let source = self.source.get(span)?;
                let count = source.chars().filter(|c| *c == NL_CHAR).count();

                if self.lines == 0 {
                    self.lines = count;
                }

                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn checked_indent(&mut self, level: isize) -> Result<usize> {
        let Some(indent) = self.indent.checked_add_signed(level) else {
            return Err(Error::new(
                self.span,
                ErrorKind::BadIndent {
                    level,
                    indent: self.indent,
                },
            ));
        };

        Ok(indent)
    }

    fn write_node(&mut self, node: &Node<'_>) -> Result<()> {
        let source = self.source.get(node.span())?;
        self.span = node.span();
        self.o.str(source).with_span(self.span)?;
        Ok(())
    }

    pub(crate) fn flush_whitespace(&mut self, preserve: bool) -> Result<()> {
        if self.use_lines && self.lines > 0 {
            self.o.lines(self.indent, self.lines.min(2))?;
            self.ws = false;
            self.use_lines = false;
            self.lines = 0;
        }

        if self.ws {
            self.o.str(WS).with_span(self.span)?;
            self.ws = false;
        }

        if !preserve {
            self.lines = 0;
            self.use_lines = false;
        }

        Ok(())
    }
}
