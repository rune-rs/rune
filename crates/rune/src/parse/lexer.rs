use core::fmt;
use core::mem::take;

use crate::alloc::{self, Vec, VecDeque};
use crate::ast;
use crate::ast::Span;
use crate::compile::{self, ErrorKind};
use crate::SourceId;

/// Lexer for the rune language.
#[derive(Debug)]
pub struct Lexer<'a> {
    /// The source identifier of the lexed data.
    source_id: SourceId,
    /// Source iterator.
    iter: SourceIter<'a>,
    /// Current lexer mode.
    modes: LexerModes,
    /// Buffered tokens.
    buffer: VecDeque<ast::Token>,
    /// If the lexer should try and lex a shebang.
    shebang: bool,
}

impl<'a> Lexer<'a> {
    /// Construct a new lexer over the given source.
    pub(crate) fn new(source: &'a str, source_id: SourceId, shebang: bool) -> Self {
        Self {
            iter: SourceIter::new(source),
            source_id,
            modes: LexerModes::default(),
            buffer: VecDeque::new(),
            shebang,
        }
    }

    /// Access the span of the lexer.
    pub(crate) fn span(&self) -> Span {
        self.iter.span_to_len(0)
    }

    /// Denote whether the next sequence of characters begin a doc comment.
    ///
    /// The lexer should have just identified a regular comment (`//_`, `/*_`, where _ is the
    /// cursor's current position).
    ///
    /// Returns a tuple (doc, inner), referring to if a doc comment was found and if the ! character
    /// was used to denote an inner comment.
    fn check_doc_comment(&mut self, ch: char) -> (bool, bool) {
        match self.iter.peek() {
            Some(c) if c == ch => {
                // The character following this must not be another of the provided character.
                // If it is, it's probably a separator, not a doc comment.
                match self.iter.peek2() {
                    Some(c) if c == ch => (false, false),
                    _ => (true, false),
                }
            }
            Some('!') => (true, true),
            _ => (false, false),
        }
    }

    fn emit_doc_attribute(
        &mut self,
        inner: bool,
        span: Span,
        docstring_span: Span,
    ) -> alloc::Result<()> {
        // outer: #[doc = ...]
        // inner: #![doc = ...]

        self.buffer
            .try_push_back(ast::Token { kind: K![#], span })?;

        if inner {
            self.buffer
                .try_push_back(ast::Token { kind: K![!], span })?;
        }

        self.buffer.try_push_back(ast::Token {
            kind: K!['['],
            span,
        })?;

        self.buffer.try_push_back(ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Doc)),
            span,
        })?;

        self.buffer
            .try_push_back(ast::Token { kind: K![=], span })?;

        self.buffer.try_push_back(ast::Token {
            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                source_id: self.source_id,
                escaped: false,
                wrapped: false,
            })),
            span: docstring_span,
        })?;

        self.buffer.try_push_back(ast::Token {
            kind: K![']'],
            span,
        })?;

        Ok(())
    }

    fn emit_builtin_attribute(&mut self, span: Span) -> alloc::Result<()> {
        self.buffer
            .try_push_back(ast::Token { kind: K![#], span })?;

        self.buffer.try_push_back(ast::Token {
            kind: K!['['],
            span,
        })?;

        self.buffer.try_push_back(ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::BuiltIn)),
            span,
        })?;

        self.buffer.try_push_back(ast::Token {
            kind: K!['('],
            span,
        })?;

        self.buffer.try_push_back(ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Literal)),
            span,
        })?;

        self.buffer.try_push_back(ast::Token {
            kind: K![')'],
            span,
        })?;

        self.buffer.try_push_back(ast::Token {
            kind: K![']'],
            span,
        })?;

        Ok(())
    }

    fn next_ident(&mut self, start: usize) -> compile::Result<Option<ast::Token>> {
        while let Some(c) = self.iter.peek() {
            if !matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9') {
                break;
            }

            self.iter.next();
        }

        match self.iter.source_from(start) {
            ("_", span) => Ok(Some(ast::Token {
                span,
                kind: ast::Kind::Underscore,
            })),
            (ident, span) => {
                let kind = ast::Kind::from_keyword(ident)
                    .unwrap_or(ast::Kind::Ident(ast::LitSource::Text(self.source_id)));
                Ok(Some(ast::Token { kind, span }))
            }
        }
    }

    /// Consume a number literal.
    fn next_number_literal(
        &mut self,
        c: char,
        start: usize,
    ) -> compile::Result<Option<ast::Token>> {
        let (base, number_start) = 'ok: {
            if let ('0', Some(m)) = (c, self.iter.peek()) {
                let number = match m {
                    'x' => ast::NumberBase::Hex,
                    'b' => ast::NumberBase::Binary,
                    'o' => ast::NumberBase::Octal,
                    _ => break 'ok (ast::NumberBase::Decimal, start),
                };

                self.iter.next();
                (number, self.iter.pos())
            } else {
                (ast::NumberBase::Decimal, start)
            }
        };

        let mut is_fractional = false;
        let mut has_exponent = false;

        let mut split = None;

        while let Some(c) = self.iter.peek() {
            match c {
                // NB: We need to avoid exponent check for hex number bases,
                // since 'e' is a legal hex literal.
                'e' if !has_exponent && !matches!(base, ast::NumberBase::Hex) => {
                    self.iter.next();
                    has_exponent = true;
                    is_fractional = true;

                    // Negative exponent.
                    if matches!(self.iter.peek(), Some('-')) {
                        self.iter.next();
                    }
                }
                '.' if !is_fractional => {
                    if let Some(p2) = self.iter.peek2() {
                        // NB: only skip if the next peek matches:
                        // * the beginning of an ident.
                        // * `..`, which is a range expression.
                        //
                        // Our goal is otherwise to consume as much alphanumeric
                        // content as possible to provide better diagnostics.
                        // But we must treat these cases differently since field
                        // accesses might be instance fn calls, and range
                        // expressions should work.
                        if matches!(p2, 'a'..='z' | 'A'..='Z' | '_' | '.') {
                            break;
                        }
                    }

                    self.iter.next();
                    is_fractional = true;
                }
                // NB: Allows for underscores to pass through number literals,
                // so that they can be used to break up large numbers in a
                // natural manner.
                '_' => {
                    self.iter.next();
                }
                c if c.is_alphanumeric() => {
                    if split.is_none()
                        && matches!((c, base), ('u' | 'i', _) | ('f', ast::NumberBase::Decimal))
                    {
                        split = Some(self.iter.pos());
                    }

                    self.iter.next();
                }
                _ => break,
            }
        }

        let end = split.unwrap_or(self.iter.pos());

        Ok(Some(ast::Token {
            kind: ast::Kind::Number(ast::NumberSource::Text(ast::NumberText {
                source_id: self.source_id,
                is_fractional,
                base,
                number: Span::new(number_start, end),
                suffix: Span::new(end, self.iter.pos()),
            })),
            span: self.iter.span_to_pos(start),
        }))
    }

    /// Consume a string literal.
    fn next_char_or_label(&mut self, start: usize) -> compile::Result<Option<ast::Token>> {
        let mut is_label = true;
        let mut count = 0;

        while let Some((s, c)) = self.iter.peek_with_pos() {
            match c {
                '\\' => {
                    self.iter.next();

                    if self.iter.next().is_none() {
                        return Err(compile::Error::new(
                            self.iter.span_to_pos(s),
                            ErrorKind::ExpectedEscape,
                        ));
                    }

                    is_label = false;
                    count += 1;
                }
                '\'' => {
                    is_label = false;
                    self.iter.next();
                    break;
                }
                // components of labels.
                '0'..='9' | 'a'..='z' => {
                    self.iter.next();
                    count += 1;
                }
                c if c.is_control() => {
                    let span = self.iter.span_to_pos(start);
                    return Err(compile::Error::new(span, ErrorKind::UnterminatedCharLit));
                }
                _ if is_label && count > 0 => {
                    break;
                }
                _ => {
                    is_label = false;
                    self.iter.next();
                    count += 1;
                }
            }
        }

        if count == 0 {
            let span = self.iter.span_to_len(start);

            if !is_label {
                return Err(compile::Error::new(span, ErrorKind::ExpectedCharClose));
            }

            return Err(compile::Error::new(span, ErrorKind::ExpectedCharOrLabel));
        }

        if is_label {
            Ok(Some(ast::Token {
                kind: ast::Kind::Label(ast::LitSource::Text(self.source_id)),
                span: self.iter.span_to_pos(start),
            }))
        } else {
            Ok(Some(ast::Token {
                kind: ast::Kind::Char(ast::CopySource::Text(self.source_id)),
                span: self.iter.span_to_pos(start),
            }))
        }
    }

    /// Consume a string literal.
    fn next_lit_byte(&mut self, start: usize) -> compile::Result<Option<ast::Token>> {
        loop {
            let (s, c) = match self.iter.next_with_pos() {
                Some(c) => c,
                None => {
                    return Err(compile::Error::new(
                        self.iter.span_to_pos(start),
                        ErrorKind::ExpectedByteClose,
                    ));
                }
            };

            match c {
                '\\' => {
                    if self.iter.next().is_none() {
                        return Err(compile::Error::new(
                            self.iter.span_to_pos(s),
                            ErrorKind::ExpectedEscape,
                        ));
                    }
                }
                '\'' => {
                    break;
                }
                c if c.is_control() => {
                    let span = self.iter.span_to_pos(start);
                    return Err(compile::Error::new(span, ErrorKind::UnterminatedByteLit));
                }
                _ => (),
            }
        }

        Ok(Some(ast::Token {
            kind: ast::Kind::Byte(ast::CopySource::Text(self.source_id)),
            span: self.iter.span_to_pos(start),
        }))
    }

    /// Consume a string literal.
    fn next_str(
        &mut self,
        start: usize,
        error_kind: impl FnOnce() -> ErrorKind + Copy,
        kind: impl FnOnce(ast::StrSource) -> ast::Kind,
    ) -> compile::Result<Option<ast::Token>> {
        let mut escaped = false;

        loop {
            let (s, c) = match self.iter.next_with_pos() {
                Some(next) => next,
                None => {
                    return Err(compile::Error::new(
                        self.iter.span_to_pos(start),
                        error_kind(),
                    ));
                }
            };

            match c {
                '"' => break,
                '\\' => {
                    if self.iter.next().is_none() {
                        return Err(compile::Error::new(
                            self.iter.span_to_pos(s),
                            ErrorKind::ExpectedEscape,
                        ));
                    }

                    escaped = true;
                }
                _ => (),
            }
        }

        Ok(Some(ast::Token {
            kind: kind(ast::StrSource::Text(ast::StrText {
                source_id: self.source_id,
                escaped,
                wrapped: true,
            })),
            span: self.iter.span_to_pos(start),
        }))
    }

    /// Consume the entire line.
    fn consume_line(&mut self) {
        while !matches!(self.iter.next(), Some('\n') | None) {}
    }

    /// Consume whitespace.
    fn consume_whitespace(&mut self) {
        while let Some(c) = self.iter.peek() {
            if !c.is_whitespace() {
                break;
            }

            self.iter.next();
        }
    }

    /// Consume a multiline comment and indicate if it's terminated correctly.
    fn consume_multiline_comment(&mut self) -> bool {
        self.iter.next();
        self.iter.next();

        let mut cur = self.iter.next();

        while let Some(a) = cur {
            cur = self.iter.next();

            if matches!((a, cur), ('*', Some('/'))) {
                return true;
            }
        }

        false
    }

    fn template_next(&mut self) -> compile::Result<()> {
        let start = self.iter.pos();
        let mut escaped = false;

        while let Some((s, c)) = self.iter.peek_with_pos() {
            match c {
                '$' => {
                    let expressions = self.modes.expression_count(&self.iter, start)?;

                    let span = self.iter.span_to_pos(start);
                    let had_string = start != self.iter.pos();
                    let start = self.iter.pos();

                    self.iter.next();

                    match self.iter.next_with_pos() {
                        Some((_, '{')) => (),
                        Some((start, c)) => {
                            let span = self.iter.span_to_pos(start);
                            return Err(compile::Error::new(span, ErrorKind::UnexpectedChar { c }));
                        }
                        None => {
                            let span = self.iter.span_to_len(start);
                            return Err(compile::Error::new(span, ErrorKind::UnexpectedEof));
                        }
                    }

                    if had_string {
                        if *expressions > 0 {
                            self.buffer.try_push_back(ast::Token {
                                kind: ast::Kind::Comma,
                                span,
                            })?;
                        }

                        self.buffer.try_push_back(ast::Token {
                            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                                source_id: self.source_id,
                                escaped: take(&mut escaped),
                                wrapped: false,
                            })),
                            span,
                        })?;

                        *expressions += 1;
                    }

                    if *expressions > 0 {
                        self.buffer.try_push_back(ast::Token {
                            kind: ast::Kind::Comma,
                            span: self.iter.span_to_pos(start),
                        })?;
                    }

                    self.modes.push(LexerMode::Default(1))?;
                    return Ok(());
                }
                '\\' => {
                    self.iter.next();

                    if self.iter.next().is_none() {
                        return Err(compile::Error::new(
                            self.iter.span_to_pos(s),
                            ErrorKind::ExpectedEscape,
                        ));
                    }

                    escaped = true;
                }
                '`' => {
                    let span = self.iter.span_to_pos(start);
                    let had_string = start != self.iter.pos();
                    let start = self.iter.pos();
                    self.iter.next();

                    let expressions = self.modes.expression_count(&self.iter, start)?;

                    if had_string {
                        if *expressions > 0 {
                            self.buffer.try_push_back(ast::Token {
                                kind: ast::Kind::Comma,
                                span,
                            })?;
                        }

                        self.buffer.try_push_back(ast::Token {
                            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                                source_id: self.source_id,
                                escaped: take(&mut escaped),
                                wrapped: false,
                            })),
                            span,
                        })?;

                        *expressions += 1;
                    }

                    self.buffer.try_push_back(ast::Token {
                        kind: K![')'],
                        span: self.iter.span_to_pos(start),
                    })?;

                    self.buffer.try_push_back(ast::Token {
                        kind: ast::Kind::Close(ast::Delimiter::Empty),
                        span: self.iter.span_to_pos(start),
                    })?;

                    let expressions = *expressions;
                    self.modes
                        .pop(&self.iter, LexerMode::Template(expressions))?;

                    return Ok(());
                }
                _ => {
                    self.iter.next();
                }
            }
        }

        Err(compile::Error::new(
            self.iter.point_span(),
            ErrorKind::UnexpectedEof,
        ))
    }

    /// Consume the next token from the lexer.
    #[allow(clippy::should_implement_trait)]
    pub(crate) fn next(&mut self) -> compile::Result<Option<ast::Token>> {
        'outer: loop {
            if let Some(token) = self.buffer.pop_front() {
                return Ok(Some(token));
            }

            let mode = self.modes.last();

            let level = match mode {
                LexerMode::Template(..) => {
                    self.template_next()?;
                    continue;
                }
                LexerMode::Default(level) => level,
            };

            let (start, c) = match self.iter.next_with_pos() {
                Some(next) => next,
                None => {
                    self.modes.pop(&self.iter, LexerMode::Default(0))?;
                    return Ok(None);
                }
            };

            // Added here specifically to avoid skipping over leading whitespace
            // tokens just below. We only ever want to parse shebangs which are
            // the first two leading characters in any input.
            if self.shebang {
                self.shebang = false;

                if matches!((c, self.iter.peek()), ('#', Some('!'))) {
                    self.consume_line();

                    return Ok(Some(ast::Token {
                        kind: ast::Kind::Shebang(ast::LitSource::Text(self.source_id)),
                        span: self.iter.span_to_pos(start),
                    }));
                }
            }

            if char::is_whitespace(c) {
                self.consume_whitespace();

                return Ok(Some(ast::Token {
                    kind: ast::Kind::Whitespace,
                    span: self.iter.span_to_pos(start),
                }));
            }

            // This loop is useful, at least until it's rewritten.
            #[allow(clippy::never_loop)]
            let kind = loop {
                if let Some(c2) = self.iter.peek() {
                    match (c, c2) {
                        ('+', '=') => {
                            self.iter.next();
                            break ast::Kind::PlusEq;
                        }
                        ('-', '=') => {
                            self.iter.next();
                            break ast::Kind::DashEq;
                        }
                        ('*', '=') => {
                            self.iter.next();
                            break ast::Kind::StarEq;
                        }
                        ('/', '=') => {
                            self.iter.next();
                            break ast::Kind::SlashEq;
                        }
                        ('%', '=') => {
                            self.iter.next();
                            break ast::Kind::PercEq;
                        }
                        ('&', '=') => {
                            self.iter.next();
                            break ast::Kind::AmpEq;
                        }
                        ('^', '=') => {
                            self.iter.next();
                            break ast::Kind::CaretEq;
                        }
                        ('|', '=') => {
                            self.iter.next();
                            break ast::Kind::PipeEq;
                        }
                        ('/', '/') => {
                            self.iter.next();
                            let (doc, inner) = self.check_doc_comment('/');
                            self.consume_line();
                            if doc {
                                // docstring span drops the first 3 characters (/// or //!)
                                let span = self.iter.span_to_pos(start);
                                self.emit_doc_attribute(inner, span, span.trim_start(3))?;
                                continue 'outer;
                            } else {
                                break ast::Kind::Comment;
                            }
                        }
                        ('/', '*') => {
                            self.iter.next();
                            let (doc, inner) = self.check_doc_comment('*');
                            let term = self.consume_multiline_comment();
                            if !term {
                                break ast::Kind::MultilineComment(false);
                            }

                            if doc {
                                // docstring span drops the first 3 characters (/** or /*!)
                                // drop the last two characters to remove */
                                let span = self.iter.span_to_pos(start);
                                self.emit_doc_attribute(
                                    inner,
                                    span,
                                    span.trim_start(3).trim_end(2),
                                )?;
                                continue 'outer;
                            } else {
                                break ast::Kind::MultilineComment(true);
                            }
                        }
                        (':', ':') => {
                            self.iter.next();
                            break ast::Kind::ColonColon;
                        }
                        ('<', '=') => {
                            self.iter.next();
                            break ast::Kind::LtEq;
                        }
                        ('>', '=') => {
                            self.iter.next();
                            break ast::Kind::GtEq;
                        }
                        ('=', '=') => {
                            self.iter.next();
                            break ast::Kind::EqEq;
                        }
                        ('!', '=') => {
                            self.iter.next();
                            break ast::Kind::BangEq;
                        }
                        ('&', '&') => {
                            self.iter.next();
                            break ast::Kind::AmpAmp;
                        }
                        ('|', '|') => {
                            self.iter.next();
                            break ast::Kind::PipePipe;
                        }
                        ('<', '<') => {
                            self.iter.next();

                            break if matches!(self.iter.peek(), Some('=')) {
                                self.iter.next();
                                ast::Kind::LtLtEq
                            } else {
                                ast::Kind::LtLt
                            };
                        }
                        ('>', '>') => {
                            self.iter.next();

                            break if matches!(self.iter.peek(), Some('=')) {
                                self.iter.next();
                                ast::Kind::GtGtEq
                            } else {
                                ast::Kind::GtGt
                            };
                        }
                        ('.', '.') => {
                            self.iter.next();

                            break if matches!(self.iter.peek(), Some('=')) {
                                self.iter.next();
                                ast::Kind::DotDotEq
                            } else {
                                ast::Kind::DotDot
                            };
                        }
                        ('=', '>') => {
                            self.iter.next();
                            break ast::Kind::Rocket;
                        }
                        ('-', '>') => {
                            self.iter.next();
                            break ast::Kind::Arrow;
                        }
                        ('b', '\'') => {
                            self.iter.next();
                            self.iter.next();
                            return self.next_lit_byte(start);
                        }
                        ('b', '"') => {
                            self.iter.next();
                            return self.next_str(
                                start,
                                || ErrorKind::UnterminatedByteStrLit,
                                ast::Kind::ByteStr,
                            );
                        }
                        _ => (),
                    }
                }

                break match c {
                    '(' => ast::Kind::Open(ast::Delimiter::Parenthesis),
                    ')' => ast::Kind::Close(ast::Delimiter::Parenthesis),
                    '{' => {
                        if level > 0 {
                            self.modes.push(LexerMode::Default(level + 1))?;
                        }

                        ast::Kind::Open(ast::Delimiter::Brace)
                    }
                    '}' => {
                        if level > 0 {
                            self.modes.pop(&self.iter, LexerMode::Default(level))?;

                            // NB: end of expression in template.
                            if level == 1 {
                                let expressions = self.modes.expression_count(&self.iter, start)?;
                                *expressions += 1;
                                continue 'outer;
                            }
                        }

                        ast::Kind::Close(ast::Delimiter::Brace)
                    }
                    '[' => ast::Kind::Open(ast::Delimiter::Bracket),
                    ']' => ast::Kind::Close(ast::Delimiter::Bracket),
                    ',' => ast::Kind::Comma,
                    ':' => ast::Kind::Colon,
                    '#' => ast::Kind::Pound,
                    '.' => ast::Kind::Dot,
                    ';' => ast::Kind::SemiColon,
                    '=' => ast::Kind::Eq,
                    '+' => ast::Kind::Plus,
                    '-' => ast::Kind::Dash,
                    '/' => ast::Kind::Div,
                    '*' => ast::Kind::Star,
                    '&' => ast::Kind::Amp,
                    '>' => ast::Kind::Gt,
                    '<' => ast::Kind::Lt,
                    '!' => ast::Kind::Bang,
                    '?' => ast::Kind::QuestionMark,
                    '|' => ast::Kind::Pipe,
                    '%' => ast::Kind::Perc,
                    '^' => ast::Kind::Caret,
                    '@' => ast::Kind::At,
                    '$' => ast::Kind::Dollar,
                    '~' => ast::Kind::Tilde,
                    '_' | 'a'..='z' | 'A'..='Z' => {
                        return self.next_ident(start);
                    }
                    '0'..='9' => {
                        return self.next_number_literal(c, start);
                    }
                    '"' => {
                        return self.next_str(
                            start,
                            || ErrorKind::UnterminatedStrLit,
                            ast::Kind::Str,
                        );
                    }
                    '`' => {
                        let span = self.iter.span_to_pos(start);

                        self.buffer.try_push_back(ast::Token {
                            kind: ast::Kind::Open(ast::Delimiter::Empty),
                            span,
                        })?;

                        self.emit_builtin_attribute(span)?;

                        self.buffer.try_push_back(ast::Token {
                            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Template)),
                            span,
                        })?;

                        self.buffer
                            .try_push_back(ast::Token { kind: K![!], span })?;

                        self.buffer.try_push_back(ast::Token {
                            kind: K!['('],
                            span,
                        })?;

                        self.modes.push(LexerMode::Template(0))?;
                        continue 'outer;
                    }
                    '\'' => {
                        return self.next_char_or_label(start);
                    }
                    _ => {
                        let span = self.iter.span_to_pos(start);
                        return Err(compile::Error::new(span, ErrorKind::UnexpectedChar { c }));
                    }
                };
            };

            return Ok(Some(ast::Token {
                kind,
                span: self.iter.span_to_pos(start),
            }));
        }
    }
}

#[derive(Debug, Clone)]
struct SourceIter<'a> {
    source: &'a str,
    cursor: usize,
}

impl<'a> SourceIter<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, cursor: 0 }
    }

    /// Get the current character position of the iterator.
    #[inline]
    fn pos(&self) -> usize {
        self.cursor
    }

    /// Get the source from the given start, to the current position.
    fn source_from(&self, start: usize) -> (&'a str, Span) {
        let end = self.pos();
        let span = Span::new(start, end);
        (&self.source[start..end], span)
    }

    /// Get the current point span.
    fn point_span(&self) -> Span {
        Span::point(self.pos())
    }

    /// Get the span from the given start, to the current position.
    fn span_to_pos(&self, start: usize) -> Span {
        Span::new(start, self.pos())
    }

    /// Get the end span from the given start to the end of the source.
    fn span_to_len(&self, start: usize) -> Span {
        Span::new(start, self.source.len())
    }

    /// Peek the next index.
    fn peek(&self) -> Option<char> {
        self.source.get(self.cursor..)?.chars().next()
    }

    /// Peek the next next char.
    fn peek2(&self) -> Option<char> {
        let mut it = self.source.get(self.cursor..)?.chars();
        it.next()?;
        it.next()
    }

    /// Peek the next character with position.
    fn peek_with_pos(&self) -> Option<(usize, char)> {
        self.clone().next_with_pos()
    }

    /// Next with position.
    fn next_with_pos(&mut self) -> Option<(usize, char)> {
        let p = self.pos();
        let c = self.next()?;
        Some((p, c))
    }
}

impl Iterator for SourceIter<'_> {
    type Item = char;

    /// Consume the next character.
    fn next(&mut self) -> Option<Self::Item> {
        let c = self.source.get(self.cursor..)?.chars().next()?;
        self.cursor += c.len_utf8();
        Some(c)
    }
}

struct WithCharIndex<'s, 'a> {
    iter: &'s mut SourceIter<'a>,
}

impl Iterator for WithCharIndex<'_, '_> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.iter.pos();
        Some((pos, self.iter.next()?))
    }
}

#[derive(Debug, Default)]
struct LexerModes {
    modes: Vec<LexerMode>,
}

impl LexerModes {
    /// Get the last mode.
    fn last(&self) -> LexerMode {
        self.modes.last().copied().unwrap_or_default()
    }

    /// Push the given lexer mode.
    fn push(&mut self, mode: LexerMode) -> alloc::Result<()> {
        self.modes.try_push(mode)
    }

    /// Pop the expected lexer mode.
    fn pop(&mut self, iter: &SourceIter<'_>, expected: LexerMode) -> compile::Result<()> {
        let actual = self.modes.pop().unwrap_or_default();

        if actual != expected {
            return Err(compile::Error::new(
                iter.point_span(),
                ErrorKind::BadLexerMode { actual, expected },
            ));
        }

        Ok(())
    }

    /// Get the expression count.
    fn expression_count<'a>(
        &'a mut self,
        iter: &SourceIter<'_>,
        start: usize,
    ) -> compile::Result<&'a mut usize> {
        match self.modes.last_mut() {
            Some(LexerMode::Template(expression)) => Ok(expression),
            _ => {
                let span = iter.span_to_pos(start);
                Err(compile::Error::new(
                    span,
                    ErrorKind::BadLexerMode {
                        actual: LexerMode::default(),
                        expected: LexerMode::Template(0),
                    },
                ))
            }
        }
    }
}

/// The mode of the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexerMode {
    /// Default mode, boolean indicating if we are inside a template or not.
    Default(usize),
    /// We are parsing a template string.
    Template(usize),
}

impl Default for LexerMode {
    fn default() -> Self {
        Self::Default(0)
    }
}

impl fmt::Display for LexerMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexerMode::Default(..) => {
                write!(f, "default")?;
            }
            LexerMode::Template(..) => {
                write!(f, "template")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use crate::{ast, SourceId};

    macro_rules! test_lexer {
        ($source:expr $(, $pat:pat)* $(,)?) => {{
            let mut it = Lexer::new($source, SourceId::empty(), false);

            #[allow(unused_assignments)]
            {
                let mut n = 0;

                $(
                    match it.next().unwrap().expect("expected token") {
                        $pat => (),
                        #[allow(unreachable_patterns)]
                        other => {
                            panic!("\nGot bad token #{}.\nExpected: `{}`\nBut got: {:?}", n, stringify!($pat), other);
                        }
                    }

                    n += 1;
                )*
            }

            assert_eq!(it.next().unwrap(), None);
        }}
    }

    #[test]
    fn test_number_literals() {
        test_lexer! {
            "(10)",
            ast::Token {
                span: span!(0, 1),
                kind: ast::Kind::Open(ast::Delimiter::Parenthesis),
            },
            ast::Token {
                span: span!(1, 3),
                kind: ast::Kind::Number(ast::NumberSource::Text(ast::NumberText {
                    source_id: SourceId::EMPTY,
                    is_fractional: false,
                    base: ast::NumberBase::Decimal,
                    number: span!(1, 3),
                    suffix: span!(3, 3),
                })),
            },
            ast::Token {
                span: span!(3, 4),
                kind: ast::Kind::Close(ast::Delimiter::Parenthesis),
            },
        };

        test_lexer! {
            "(10.)",
            _,
            ast::Token {
                span: span!(1, 4),
                kind: ast::Kind::Number(ast::NumberSource::Text(ast::NumberText {
                    source_id: SourceId::EMPTY,
                    is_fractional: true,
                    base: ast::NumberBase::Decimal,
                    number: span!(1, 4),
                    suffix: span!(4, 4),
                })),
            },
            _,
        };
    }

    #[test]
    fn test_char_literal() {
        test_lexer! {
            "'a'",
            ast::Token {
                span: span!(0, 3),
                kind: ast::Kind::Char(ast::CopySource::Text(SourceId::EMPTY)),
            }
        };

        test_lexer! {
            "'\\u{abcd}'",
            ast::Token {
                span: span!(0, 10),
                kind: ast::Kind::Char(ast::CopySource::Text(SourceId::EMPTY)),
            }
        };
    }

    #[test]
    fn test_label() {
        test_lexer! {
            "'asdf 'a' \"foo bar\"",
            ast::Token {
                span: span!(0, 5),
                kind: ast::Kind::Label(ast::LitSource::Text(SourceId::EMPTY)),
            },
            ast::Token {
                span: span!(5, 6),
                kind: ast::Kind::Whitespace,
            },
            ast::Token {
                span: span!(6, 9),
                kind: ast::Kind::Char(ast::CopySource::Text(SourceId::EMPTY)),
            },
            ast::Token {
                span: span!(9, 10),
                kind: ast::Kind::Whitespace,
            },
            ast::Token {
                span: span!(10, 19),
                kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText { source_id: SourceId::EMPTY, escaped: false, wrapped: true })),
            }
        };
    }

    #[test]
    fn test_operators() {
        test_lexer! {
            "+ += - -= * *= / /=",
            ast::Token {
                span: span!(0, 1),
                kind: ast::Kind::Plus,
            },
            ast::Token {
                span: span!(1, 2),
                kind: ast::Kind::Whitespace,
            },
            ast::Token {
                span: span!(2, 4),
                kind: ast::Kind::PlusEq,
            },
            ast::Token {
                span: span!(4, 5),
                kind: ast::Kind::Whitespace,
            },
            ast::Token {
                span: span!(5, 6),
                kind: ast::Kind::Dash,
            },
            ast::Token {
                span: span!(6, 7),
                kind: ast::Kind::Whitespace,
            },
            ast::Token {
                span: span!(7, 9),
                kind: ast::Kind::DashEq,
            },
            ast::Token {
                span: span!(9, 10),
                kind: ast::Kind::Whitespace,
            },
            ast::Token {
                span: span!(10, 11),
                kind: ast::Kind::Star,
            },
            ast::Token {
                span: span!(11, 12),
                kind: ast::Kind::Whitespace,
            },
            ast::Token {
                span: span!(12, 14),
                kind: ast::Kind::StarEq,
            },
            ast::Token {
                span: span!(14, 15),
                kind: ast::Kind::Whitespace,
            },
            ast::Token {
                span: span!(15, 16),
                kind: ast::Kind::Div,
            },
            ast::Token {
                span: span!(16, 17),
                kind: ast::Kind::Whitespace,
            },
            ast::Token {
                span: span!(17, 19),
                kind: ast::Kind::SlashEq,
            }
        };
    }

    #[test]
    fn test_idents() {
        test_lexer! {
            "a.checked_div(10)",
            ast::Token {
                span: span!(0, 1),
                kind: ast::Kind::Ident(ast::LitSource::Text(SourceId::EMPTY)),
            },
            ast::Token {
                span: span!(1, 2),
                kind: ast::Kind::Dot,
            },
            ast::Token {
                span: span!(2, 13),
                kind: ast::Kind::Ident(ast::LitSource::Text(SourceId::EMPTY)),
            },
            ast::Token {
                span: span!(13, 14),
                kind: ast::Kind::Open(ast::Delimiter::Parenthesis),
            },
            ast::Token {
                span: span!(14, 16),
                kind: ast::Kind::Number(ast::NumberSource::Text(ast::NumberText {
                    source_id: SourceId::EMPTY,
                    is_fractional: false,
                    base: ast::NumberBase::Decimal,
                    number: span!(14, 16),
                    suffix: span!(16, 16),
                })),
            },
            ast::Token {
                span: span!(16, 17),
                kind: ast::Kind::Close(ast::Delimiter::Parenthesis),
            },
        };
    }

    #[test]
    fn test_doc_strings() {
        test_lexer! {
            "//! inner\n\
             /// \"quoted\"",
            ast::Token {
                kind: K![#],
                span: span!(0, 10)
            },
            ast::Token {
                kind: K![!],
                span: span!(0, 10)
            },
            ast::Token {
                kind: K!['['],
                span: span!(0, 10)
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Doc)),
                span: span!(0, 10)
            },
            ast::Token {
                kind: K![=],
                span: span!(0, 10)
            },
            ast::Token {
                kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                    source_id: SourceId::EMPTY,
                    escaped: false,
                    wrapped: false,
                })),
                span: span!(3, 10)
            },
            ast::Token {
                kind: K![']'],
                span: span!(0, 10)
            },
            ast::Token {
                kind: K![#],
                span: span!(10, 22)
            },
            ast::Token {
                kind: K!['['],
                span: span!(10, 22)
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Doc)),
                span: span!(10, 22)
            },
            ast::Token {
                kind: K![=],
                span: span!(10, 22)
            },
            ast::Token {
                kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                    source_id: SourceId::EMPTY,
                    escaped: false,
                    wrapped: false,
                })),
                span: span!(13, 22)
            },
            ast::Token {
                kind: K![']'],
                span: span!(10, 22)
            },
        };
    }

    #[test]
    fn test_multiline_docstring() {
        test_lexer! {
            // /*!
            //  * inner docstr
            //  */
            // /**
            //  * docstr
            //  */
            "/*!\n * inner docstr\n */\n/**\n * docstr\n */",
            ast::Token {
                kind: K![#],
                span: span!(0, 23)
            },
            ast::Token {
                kind: K![!],
                span: span!(0, 23)
            },
            ast::Token {
                kind: K!['['],
                span: span!(0, 23)
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Doc)),
                span: span!(0, 23)
            },
            ast::Token {
                kind: K![=],
                span: span!(0, 23)
            },
            ast::Token {
                kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                    source_id: SourceId::EMPTY,
                    escaped: false,
                    wrapped: false,
                })),
                span: span!(3, 21)
            },
            ast::Token {
                kind: K![']'],
                span: span!(0, 23)
            },
            ast::Token {
                kind: ast::Kind::Whitespace,
                span: span!(23, 24)
            },
            ast::Token {
                kind: K![#],
                span: span!(24, 41)
            },
            ast::Token {
                kind: K!['['],
                span: span!(24, 41)
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Doc)),
                span: span!(24, 41)
            },
            ast::Token {
                kind: K![=],
                span: span!(24, 41)
            },
            ast::Token {
                kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                    source_id: SourceId::EMPTY,
                    escaped: false,
                    wrapped: false,
                })),
                span: span!(27, 39)
            },
            ast::Token {
                kind: K![']'],
                span: span!(24, 41)
            },
        };
    }

    #[test]
    fn test_comment_separators() {
        test_lexer! {
            "///////////////////////////////////\n\
            /*********************************/\n\
            /**********************************\n\
            *                                 *\n\
            ***********************************/",
            ast::Token {
                kind: ast::Kind::Comment,
                span: span!(0, 36)
            },
            ast::Token {
                kind: ast::Kind::MultilineComment(true),
                span: span!(36, 71)
            },
            ast::Token {
                kind: ast::Kind::Whitespace,
                span: span!(71, 72)
            },
            ast::Token {
                kind: ast::Kind::MultilineComment(true),
                span: span!(72, 180)
            },
        };
    }

    #[test]
    fn test_template_literals() {
        test_lexer! {
            "`foo ${bar} \\` baz`",
            ast::Token {
                kind: ast::Kind::Open(ast::Delimiter::Empty),
                span: span!(0, 1),
            },
            ast::Token {
                kind: K![#],
                span: span!(0, 1),
            },
            ast::Token {
                kind: K!['['],
                span: span!(0, 1),
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::BuiltIn)),
                span: span!(0, 1),
            },
            ast::Token {
                kind: K!['('],
                span: span!(0, 1),
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Literal)),
                span: span!(0, 1),
            },
            ast::Token {
                kind: K![')'],
                span: span!(0, 1),
            },
            ast::Token {
                kind: K![']'],
                span: span!(0, 1),
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Template)),
                span: span!(0, 1),
            },
            ast::Token {
                kind: ast::Kind::Bang,
                span: span!(0, 1),
            },
            ast::Token {
                kind: K!['('],
                span: span!(0, 1),
            },
            ast::Token {
                kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                    source_id: SourceId::EMPTY,
                    escaped: false,
                    wrapped: false,
                })),
                span: span!(1, 5),
            },
            ast::Token {
                kind: ast::Kind::Comma,
                span: span!(5, 7),
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::Text(SourceId::EMPTY)),
                span: span!(7, 10),
            },
            ast::Token {
                kind: ast::Kind::Comma,
                span: span!(11, 18),
            },
            ast::Token {
                kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                    source_id: SourceId::EMPTY,
                    escaped: true,
                    wrapped: false,
                })),
                span: span!(11, 18),
            },
            ast::Token {
                kind: K![')'],
                span: span!(18, 19),
            },
            ast::Token {
                kind: ast::Kind::Close(ast::Delimiter::Empty),
                span: span!(18, 19),
            },
        };
    }

    #[test]
    fn test_template_literals_multi() {
        test_lexer! {
            "`foo ${bar} ${baz}`",
            ast::Token {
                kind: ast::Kind::Open(ast::Delimiter::Empty),
                span: span!(0, 1),
            },
            ast::Token {
                kind: K![#],
                span: span!(0, 1),
            },
            ast::Token {
                kind: K!['['],
                span: span!(0, 1),
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::BuiltIn)),
                span: span!(0, 1),
            },
            ast::Token {
                kind: K!['('],
                span: span!(0, 1),
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Literal)),
                span: span!(0, 1),
            },
            ast::Token {
                kind: K![')'],
                span: span!(0, 1),
            },
            ast::Token {
                kind: K![']'],
                span: span!(0, 1),
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Template)),
                span: span!(0, 1),
            },
            ast::Token {
                kind: ast::Kind::Bang,
                span: span!(0, 1),
            },
            ast::Token {
                kind: K!['('],
                span: span!(0, 1),
            },
            ast::Token {
                kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                    source_id: SourceId::EMPTY,
                    escaped: false,
                    wrapped: false,
                })),
                span: span!(1, 5),
            },
            ast::Token {
                kind: ast::Kind::Comma,
                span: span!(5, 7),
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::Text(SourceId::EMPTY)),
                span: span!(7, 10),
            },
            ast::Token {
                kind: ast::Kind::Comma,
                span: span!(11, 12),
            },
            ast::Token {
                kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                    source_id: SourceId::EMPTY,
                    escaped: false,
                    wrapped: false,
                })),
                span: span!(11, 12),
            },
            ast::Token {
                kind: ast::Kind::Comma,
                span: span!(12, 14),
            },
            ast::Token {
                kind: ast::Kind::Ident(ast::LitSource::Text(SourceId::EMPTY)),
                span: span!(14, 17),
            },
            ast::Token {
                kind: K![')'],
                span: span!(18, 19),
            },
            ast::Token {
                kind: ast::Kind::Close(ast::Delimiter::Empty),
                span: span!(18, 19),
            },
        };
    }

    #[test]
    fn test_literals() {
        test_lexer! {
            r#"b"""#,
            ast::Token {
                span: span!(0, 3),
                kind: ast::Kind::ByteStr(ast::StrSource::Text(ast::StrText {
                    source_id: SourceId::EMPTY,
                    escaped: false,
                    wrapped: true,
                })),
            },
        };

        test_lexer! {
            r#"b"hello world""#,
            ast::Token {
                span: span!(0, 14),
                kind: ast::Kind::ByteStr(ast::StrSource::Text(ast::StrText {
                    source_id: SourceId::EMPTY,
                    escaped: false,
                    wrapped: true,
                })),
            },
        };

        test_lexer! {
            "b'\\\\''",
            ast::Token {
                span: span!(0, 6),
                kind: ast::Kind::Byte(ast::CopySource::Text(SourceId::EMPTY)),
            },
        };

        test_lexer! {
            "'label 'a' b'a'",
            ast::Token {
                span: span!(0, 6),
                kind: ast::Kind::Label(ast::LitSource::Text(SourceId::EMPTY)),
            },
            ast::Token {
                span: span!(6, 7),
                kind: ast::Kind::Whitespace,
            },
            ast::Token {
                span: span!(7, 10),
                kind: ast::Kind::Char(ast::CopySource::Text(SourceId::EMPTY)),
            },
            ast::Token {
                span: span!(10, 11),
                kind: ast::Kind::Whitespace,
            },
            ast::Token {
                span: span!(11, 15),
                kind: ast::Kind::Byte(ast::CopySource::Text(SourceId::EMPTY)),
            },
        };

        test_lexer! {
            "b'a'",
            ast::Token {
                span: span!(0, 4),
                kind: ast::Kind::Byte(ast::CopySource::Text(SourceId::EMPTY)),
            },
        };

        test_lexer! {
            "b'\\n'",
            ast::Token {
                span: span!(0, 5),
                kind: ast::Kind::Byte(ast::CopySource::Text(SourceId::EMPTY)),
            },
        };
    }
}
