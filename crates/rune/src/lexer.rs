use crate::ast;
use crate::ast::utils;
use crate::{ParseError, ParseErrorKind};
use runestick::Span;

/// Lexer for the rune language.
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    cursor: usize,
    source: &'a str,
}

impl<'a> Lexer<'a> {
    /// Construct a new lexer over the given source.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rune::Lexer;
    /// use rune::ast;
    /// use runestick::Span;
    ///
    /// assert_eq! {
    ///     Lexer::new("fn").next().unwrap().unwrap(),
    ///     ast::Token {
    ///         kind: ast::Kind::Fn,
    ///         span: Span { start: 0, end: 2 },
    ///     }
    /// };
    ///
    /// assert_eq! {
    ///     Lexer::new("name").next().unwrap().unwrap(),
    ///     ast::Token {
    ///         kind: ast::Kind::Ident(ast::StringSource::Text),
    ///         span: Span { start: 0, end: 4 },
    ///     }
    /// };
    /// ```
    pub fn new(source: &'a str) -> Self {
        Self { cursor: 0, source }
    }

    /// Construct a new lexer with the given start.
    pub fn new_with_start(source: &'a str, start: usize) -> Self {
        Self {
            cursor: start,
            source,
        }
    }

    /// Access the end span of the input.
    pub fn end(&self) -> Span {
        Span::point(self.source.len())
    }

    /// Calculate the end span by peeking the next token.
    fn end_span<I, T>(&self, it: &I) -> usize
    where
        I: Iterator<Item = (usize, T)> + Clone,
    {
        it.clone()
            .next()
            .map(|(n, _)| self.cursor + n)
            .unwrap_or_else(|| self.source.len())
    }

    fn next_ident<I>(&mut self, it: &mut I, start: usize) -> Result<Option<ast::Token>, ParseError>
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        self.cursor = loop {
            break match it.clone().next() {
                Some((n, c)) => match c {
                    'a'..='z' | 'A'..='Z' | '_' | '0'..='9' => {
                        it.next();
                        continue;
                    }
                    _ => self.cursor + n,
                },
                None => self.source.len(),
            };
        };

        let ident = &self.source[start..self.cursor];

        let span = Span {
            start,
            end: self.cursor,
        };

        let kind =
            ast::Kind::from_keyword(ident).unwrap_or(ast::Kind::Ident(ast::StringSource::Text));
        Ok(Some(ast::Token { kind, span }))
    }

    /// Consume a number literal.
    fn next_number_literal<I>(
        &mut self,
        it: &mut I,
        c: char,
        start: usize,
        is_negative: bool,
    ) -> Result<Option<ast::Token>, ParseError>
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        let mut is_fractional = false;

        let base = if let ('0', Some((_, m))) = (c, it.clone().next()) {
            // This loop is useful.
            #[allow(clippy::never_loop)]
            loop {
                let number = match m {
                    'x' => ast::NumberBase::Hex,
                    'b' => ast::NumberBase::Binary,
                    'o' => ast::NumberBase::Octal,
                    _ => break ast::NumberBase::Decimal,
                };

                // consume character.
                it.next();
                break number;
            }
        } else {
            ast::NumberBase::Decimal
        };

        self.cursor = loop {
            let (n, c) = match it.next() {
                Some((n, c)) => (n, c),
                None => break self.source.len(),
            };

            match c {
                c if char::is_alphanumeric(c) => (),
                '.' if !is_fractional => {
                    is_fractional = true;

                    // char immediately following a dot should be numerical.
                    if !it.next().map(|(_, c)| c.is_numeric()).unwrap_or_default() {
                        break self.cursor + n;
                    }
                }
                _ => break self.cursor + n,
            }
        };

        Ok(Some(ast::Token {
            kind: ast::Kind::LitNumber(ast::NumberSource::Text(ast::NumberSourceText {
                is_fractional,
                is_negative,
                base,
            })),
            span: Span {
                start,
                end: self.cursor,
            },
        }))
    }

    /// Consume a string literal.
    fn next_char_or_label<I>(
        &mut self,
        it: &mut I,
        start: usize,
    ) -> Result<Option<ast::Token>, ParseError>
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        let mut is_label = true;
        let mut char_count = 0;

        self.cursor = loop {
            let (n, c) = match it.clone().next() {
                Some(c) => c,
                None => {
                    if is_label {
                        let span = Span {
                            start,
                            end: self.source.len(),
                        };

                        return Err(ParseError::new(span, ParseErrorKind::ExpectedCharClose));
                    }

                    break self.source.len();
                }
            };

            match c {
                '\\' => {
                    is_label = false;
                    it.next();
                    it.next();
                    char_count += 1;
                }
                '\'' => {
                    is_label = false;
                    it.next();
                    break self.end_span(it);
                }
                // components of labels.
                '0'..='9' | 'a'..='z' => {
                    it.next();
                    char_count += 1;
                }
                c if c.is_control() => {
                    let span = Span {
                        start,
                        end: self.cursor + n,
                    };

                    return Err(ParseError::new(span, ParseErrorKind::UnterminatedCharLit));
                }
                _ if is_label && char_count > 0 => {
                    break self.cursor + n;
                }
                _ => {
                    is_label = false;
                    it.next();
                    char_count += 1;
                }
            }
        };

        if is_label {
            Ok(Some(ast::Token {
                kind: ast::Kind::Label(ast::StringSource::Text),
                span: Span {
                    start,
                    end: self.cursor,
                },
            }))
        } else {
            Ok(Some(ast::Token {
                kind: ast::Kind::LitChar(ast::CopySource::Text),
                span: Span {
                    start,
                    end: self.cursor,
                },
            }))
        }
    }

    /// Consume a string literal.
    fn next_lit_byte<I>(
        &mut self,
        it: &mut I,
        start: usize,
    ) -> Result<Option<ast::Token>, ParseError>
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        self.cursor = loop {
            let (n, c) = match it.clone().next() {
                Some(c) => c,
                None => {
                    let span = Span {
                        start,
                        end: self.source.len(),
                    };

                    return Err(ParseError::new(span, ParseErrorKind::ExpectedByteClose));
                }
            };

            match c {
                '\\' => {
                    it.next();
                    it.next();
                }
                '\'' => {
                    it.next();
                    break self.end_span(it);
                }
                c if c.is_control() => {
                    let span = Span {
                        start,
                        end: self.cursor + n,
                    };

                    return Err(ParseError::new(span, ParseErrorKind::UnterminatedByteLit));
                }
                _ => {
                    it.next();
                }
            }
        };

        Ok(Some(ast::Token {
            kind: ast::Kind::LitByte(ast::CopySource::Text),
            span: Span {
                start,
                end: self.cursor,
            },
        }))
    }

    /// Consume a string literal.
    fn next_lit_str<I>(
        &mut self,
        it: &mut I,
        start: usize,
    ) -> Result<Option<ast::Token>, ParseError>
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        let mut escaped = false;

        self.cursor = loop {
            break match it.next() {
                Some((_, c)) => match c {
                    '"' => self.end_span(it),
                    '\\' => match it.next() {
                        Some(_) => {
                            escaped = true;
                            continue;
                        }
                        None => {
                            let span = Span {
                                start,
                                end: self.source.len(),
                            };

                            return Err(ParseError::new(
                                span,
                                ParseErrorKind::ExpectedStringEscape,
                            ));
                        }
                    },
                    _ => continue,
                },
                None => {
                    let span = Span {
                        start,
                        end: self.source.len(),
                    };

                    return Err(ParseError::new(span, ParseErrorKind::UnterminatedStrLit));
                }
            };
        };

        Ok(Some(ast::Token {
            kind: ast::Kind::LitStr(ast::LitStrSource::Text(ast::LitStrSourceText { escaped })),
            span: Span {
                start,
                end: self.cursor,
            },
        }))
    }

    /// Consume a string literal.
    fn next_lit_byte_str<I>(
        &mut self,
        it: &mut I,
        start: usize,
    ) -> Result<Option<ast::Token>, ParseError>
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        let mut escaped = false;

        self.cursor = loop {
            break match it.next() {
                Some((_, c)) => match c {
                    '"' => self.end_span(it),
                    '\\' => match it.next() {
                        Some(_) => {
                            escaped = true;
                            continue;
                        }
                        None => {
                            let span = Span {
                                start,
                                end: self.source.len(),
                            };

                            return Err(ParseError::new(
                                span,
                                ParseErrorKind::ExpectedStringEscape,
                            ));
                        }
                    },
                    _ => continue,
                },
                None => {
                    let span = Span {
                        start,
                        end: self.source.len(),
                    };

                    return Err(ParseError::new(span, ParseErrorKind::UnterminatedStrLit));
                }
            };
        };

        Ok(Some(ast::Token {
            kind: ast::Kind::LitByteStr(ast::LitByteStrSource::Text(ast::LitByteStrSourceText {
                escaped,
            })),
            span: Span {
                start,
                end: self.cursor,
            },
        }))
    }

    /// Consume a string literal.
    fn next_template<I>(
        &mut self,
        it: &mut I,
        start: usize,
    ) -> Result<Option<ast::Token>, ParseError>
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        let mut escaped = false;

        self.cursor = loop {
            break match it.next() {
                Some((n, c)) => match c {
                    '`' => self.end_span(it),
                    '{' => {
                        let span = Span::new(start, n);
                        utils::template_expr(span, it)?;
                        continue;
                    }
                    '\\' => match it.next() {
                        Some(_) => {
                            escaped = true;
                            continue;
                        }
                        None => {
                            let span = Span {
                                start,
                                end: self.source.len(),
                            };

                            return Err(ParseError::new(
                                span,
                                ParseErrorKind::ExpectedTemplateClose,
                            ));
                        }
                    },
                    _ => continue,
                },
                None => {
                    let span = Span {
                        start,
                        end: self.source.len(),
                    };

                    return Err(ParseError::new(span, ParseErrorKind::ExpectedTemplateClose));
                }
            };
        };

        Ok(Some(ast::Token {
            kind: ast::Kind::LitTemplate(ast::LitStrSource::Text(ast::LitStrSourceText {
                escaped,
            })),
            span: Span {
                start,
                end: self.cursor,
            },
        }))
    }

    /// Consume the entire line.
    fn consume_line<I>(&mut self, it: &mut I)
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        loop {
            match it.next() {
                Some((_, '\n')) | None => break,
                _ => (),
            }
        }
    }

    /// Consume the next token from the lexer.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<ast::Token>, ParseError> {
        let mut it = self.source[self.cursor..].char_indices();

        'outer: while let Some((start, c)) = it.next() {
            let start = self.cursor + start;

            if char::is_whitespace(c) {
                continue;
            }

            // This loop is useful, at least until it's rewritten.
            #[allow(clippy::never_loop)]
            let kind = loop {
                if let Some(c2) = it.clone().next().map(|(_, c)| c) {
                    match (c, c2) {
                        ('+', '=') => {
                            it.next();
                            break ast::Kind::PlusEq;
                        }
                        ('-', '=') => {
                            it.next();
                            break ast::Kind::DashEq;
                        }
                        ('*', '=') => {
                            it.next();
                            break ast::Kind::StarEq;
                        }
                        ('/', '=') => {
                            it.next();
                            break ast::Kind::SlashEq;
                        }
                        ('%', '=') => {
                            it.next();
                            break ast::Kind::PercEq;
                        }
                        ('&', '=') => {
                            it.next();
                            break ast::Kind::AmpEq;
                        }
                        ('^', '=') => {
                            it.next();
                            break ast::Kind::CaretEq;
                        }
                        ('|', '=') => {
                            it.next();
                            break ast::Kind::PipeEq;
                        }
                        ('/', '/') => {
                            self.consume_line(&mut it);
                            continue 'outer;
                        }
                        (':', ':') => {
                            it.next();
                            break ast::Kind::ColonColon;
                        }
                        ('<', '=') => {
                            it.next();
                            break ast::Kind::LtEq;
                        }
                        ('>', '=') => {
                            it.next();
                            break ast::Kind::GtEq;
                        }
                        ('=', '=') => {
                            it.next();
                            break ast::Kind::EqEq;
                        }
                        ('!', '=') => {
                            it.next();
                            break ast::Kind::BangEq;
                        }
                        ('&', '&') => {
                            it.next();
                            break ast::Kind::AmpAmp;
                        }
                        ('|', '|') => {
                            it.next();
                            break ast::Kind::PipePipe;
                        }
                        ('<', '<') => {
                            it.next();

                            break if matches!(it.next(), Some((_, '='))) {
                                it.next();
                                ast::Kind::LtLtEq
                            } else {
                                ast::Kind::LtLt
                            };
                        }
                        ('>', '>') => {
                            it.next();

                            break if matches!(it.next(), Some((_, '='))) {
                                it.next();
                                ast::Kind::GtGtEq
                            } else {
                                ast::Kind::GtGt
                            };
                        }
                        ('.', '.') => {
                            it.next();
                            break ast::Kind::DotDot;
                        }
                        ('=', '>') => {
                            it.next();
                            break ast::Kind::Rocket;
                        }
                        ('-', c @ '0'..='9') => {
                            it.next();
                            return self.next_number_literal(&mut it, c, start, true);
                        }
                        ('b', '\'') => {
                            it.next();
                            it.next();
                            return self.next_lit_byte(&mut it, start);
                        }
                        ('b', '"') => {
                            it.next();
                            it.next();
                            return self.next_lit_byte_str(&mut it, start);
                        }
                        _ => (),
                    }
                }

                break match c {
                    '(' => ast::Kind::Open(ast::Delimiter::Parenthesis),
                    ')' => ast::Kind::Close(ast::Delimiter::Parenthesis),
                    '{' => ast::Kind::Open(ast::Delimiter::Brace),
                    '}' => ast::Kind::Close(ast::Delimiter::Brace),
                    '[' => ast::Kind::Open(ast::Delimiter::Bracket),
                    ']' => ast::Kind::Close(ast::Delimiter::Bracket),
                    '_' => ast::Kind::Underscore,
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
                    'a'..='z' | 'A'..='Z' => {
                        return self.next_ident(&mut it, start);
                    }
                    '0'..='9' => {
                        return self.next_number_literal(&mut it, c, start, false);
                    }
                    '"' => {
                        return self.next_lit_str(&mut it, start);
                    }
                    '`' => {
                        return self.next_template(&mut it, start);
                    }
                    '\'' => {
                        return self.next_char_or_label(&mut it, start);
                    }
                    _ => {
                        let span = Span {
                            start,
                            end: self.end_span(&it),
                        };

                        return Err(ParseError::new(span, ParseErrorKind::UnexpectedChar { c }));
                    }
                };
            };

            self.cursor = self.end_span(&it);

            return Ok(Some(ast::Token {
                kind,
                span: Span {
                    start,
                    end: self.cursor,
                },
            }));
        }

        self.cursor = self.source.len();
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use crate::ast;
    use runestick::Span;

    macro_rules! test_lexer {
        ($source:expr $(, $pat:expr)* $(,)?) => {{
            let mut it = Lexer::new($source);
            $(assert_eq!(it.next().unwrap(), Some($pat));)*
            assert_eq!(it.next().unwrap(), None);
        }}
    }

    #[test]
    fn test_char_literal() {
        test_lexer! {
            "'a'",
            ast::Token {
                span: Span::new(0, 3),
                kind: ast::Kind::LitChar(ast::CopySource::Text),
            }
        };

        test_lexer! {
            "'\\u{abcd}'",
            ast::Token {
                span: Span::new(0, 10),
                kind: ast::Kind::LitChar(ast::CopySource::Text),
            }
        };
    }

    #[test]
    fn test_label() {
        test_lexer! {
            "'asdf 'a' \"foo bar\"",
            ast::Token {
                span: Span::new(0, 5),
                kind: ast::Kind::Label(ast::StringSource::Text),
            },
            ast::Token {
                span: Span::new(6, 9),
                kind: ast::Kind::LitChar(ast::CopySource::Text),
            },
            ast::Token {
                span: Span::new(10, 19),
                kind: ast::Kind::LitStr(ast::LitStrSource::Text(ast::LitStrSourceText { escaped: false })),
            }
        };
    }

    #[test]
    fn test_operators() {
        test_lexer! {
            "+ += - -= * *= / /=",
            ast::Token {
                span: Span::new(0, 1),
                kind: ast::Kind::Plus,
            },
            ast::Token {
                span: Span::new(2, 4),
                kind: ast::Kind::PlusEq,
            },
            ast::Token {
                span: Span::new(5, 6),
                kind: ast::Kind::Dash,
            },
            ast::Token {
                span: Span::new(7, 9),
                kind: ast::Kind::DashEq,
            },
            ast::Token {
                span: Span::new(10, 11),
                kind: ast::Kind::Star,
            },
            ast::Token {
                span: Span::new(12, 14),
                kind: ast::Kind::StarEq,
            },
            ast::Token {
                span: Span::new(15, 16),
                kind: ast::Kind::Div,
            },
            ast::Token {
                span: Span::new(17, 19),
                kind: ast::Kind::SlashEq,
            }
        };
    }

    #[test]
    fn test_idents() {
        test_lexer! {
            "a.checked_div(10)",
            ast::Token {
                span: Span::new(0, 1),
                kind: ast::Kind::Ident(ast::StringSource::Text),
            },
            ast::Token {
                span: Span::new(1, 2),
                kind: ast::Kind::Dot,
            },
            ast::Token {
                span: Span::new(2, 13),
                kind: ast::Kind::Ident(ast::StringSource::Text),
            },
            ast::Token {
                span: Span::new(13, 14),
                kind: ast::Kind::Open(ast::Delimiter::Parenthesis),
            },
            ast::Token {
                span: Span::new(14, 16),
                kind: ast::Kind::LitNumber(ast::NumberSource::Text(ast::NumberSourceText {
                    is_fractional: false,
                    is_negative: false,
                    base: ast::NumberBase::Decimal,
                })),
            },
            ast::Token {
                span: Span::new(16, 17),
                kind: ast::Kind::Close(ast::Delimiter::Parenthesis),
            },
        };
    }

    #[test]
    fn test_template_literals() {
        test_lexer! {
            "`foo {bar} \\` baz`",
            ast::Token {
                span: Span::new(0, 18),
                kind: ast::Kind::LitTemplate(ast::LitStrSource::Text(ast::LitStrSourceText { escaped: true })),
            },
        };
    }

    #[test]
    fn test_literals() {
        test_lexer! {
            r#"b"hello world""#,
            ast::Token {
                span: Span::new(0, 14),
                kind: ast::Kind::LitByteStr(ast::LitByteStrSource::Text(ast::LitByteStrSourceText {
                    escaped: false,
                })),
            },
        };

        test_lexer! {
            "b'\\\\''",
            ast::Token {
                span: Span::new(0, 6),
                kind: ast::Kind::LitByte(ast::CopySource::Text),
            },
        };

        test_lexer! {
            "'label 'a' b'a'",
            ast::Token {
                span: Span::new(0, 6),
                kind: ast::Kind::Label(ast::StringSource::Text),
            },
            ast::Token {
                span: Span::new(7, 10),
                kind: ast::Kind::LitChar(ast::CopySource::Text),
            },
            ast::Token {
                span: Span::new(11, 15),
                kind: ast::Kind::LitByte(ast::CopySource::Text),
            },
        };

        test_lexer! {
            "b'a'",
            ast::Token {
                span: Span::new(0, 4),
                kind: ast::Kind::LitByte(ast::CopySource::Text),
            },
        };

        test_lexer! {
            "b'\\n'",
            ast::Token {
                span: Span::new(0, 5),
                kind: ast::Kind::LitByte(ast::CopySource::Text),
            },
        };
    }
}
