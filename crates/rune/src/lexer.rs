use crate::ast::utils;
use crate::error::{ParseError, Result};
use crate::token::{Delimiter, Kind, LitNumber, Token};
use runestick::unit::Span;

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
    /// use rune::{Lexer, Kind, Span, Token};
    ///
    /// # fn main() -> rune::Result<()> {
    /// assert_eq! {
    ///     Lexer::new("fn").next()?.unwrap(),
    ///     Token {
    ///         kind: Kind::Fn,
    ///         span: Span { start: 0, end: 2 },
    ///     }
    /// };
    ///
    /// assert_eq! {
    ///     Lexer::new("name").next()?.unwrap(),
    ///     Token {
    ///         kind: Kind::Ident,
    ///         span: Span { start: 0, end: 4 },
    ///     }
    /// };
    /// # Ok(())
    /// # }
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

    fn next_ident<I>(&mut self, it: &mut I, start: usize) -> Result<Option<Token>, ParseError>
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

        let kind = match ident {
            "self" => Kind::Self_,
            "fn" => Kind::Fn,
            "enum" => Kind::Enum,
            "struct" => Kind::Struct,
            "let" => Kind::Let,
            "if" => Kind::If,
            "match" => Kind::Match,
            "else" => Kind::Else,
            "use" => Kind::Use,
            "while" => Kind::While,
            "for" => Kind::For,
            "loop" => Kind::Loop,
            "in" => Kind::In,
            "true" => Kind::True,
            "false" => Kind::False,
            "is" => Kind::Is,
            "not" => Kind::Not,
            "break" => Kind::Break,
            "return" => Kind::Return,
            "await" => Kind::Await,
            "async" => Kind::Async,
            "select" => Kind::Select,
            "default" => Kind::Default,
            "impl" => Kind::Impl,
            _ => Kind::Ident,
        };

        Ok(Some(Token {
            kind,
            span: Span {
                start,
                end: self.cursor,
            },
        }))
    }

    /// Consume a number literal.
    fn next_number_literal<I>(
        &mut self,
        it: &mut I,
        c: char,
        start: usize,
        is_negative: bool,
    ) -> Result<Option<Token>, ParseError>
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        let mut is_fractional = false;

        let number = if let ('0', Some((_, m))) = (c, it.clone().next()) {
            // This loop is useful.
            #[allow(clippy::never_loop)]
            loop {
                let number = match m {
                    'x' => LitNumber::Hex,
                    'b' => LitNumber::Binary,
                    'o' => LitNumber::Octal,
                    _ => break LitNumber::Decimal,
                };

                // consume character.
                it.next();
                break number;
            }
        } else {
            LitNumber::Decimal
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

        Ok(Some(Token {
            kind: Kind::LitNumber {
                is_fractional,
                is_negative,
                number,
            },
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
    ) -> Result<Option<Token>, ParseError>
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
                        return Err(ParseError::ExpectedCharClose {
                            span: Span {
                                start,
                                end: self.source.len(),
                            },
                        });
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
                    return Err(ParseError::UnterminatedCharLit {
                        span: Span {
                            start,
                            end: self.cursor + n,
                        },
                    });
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
            Ok(Some(Token {
                kind: Kind::Label,
                span: Span {
                    start,
                    end: self.cursor,
                },
            }))
        } else {
            Ok(Some(Token {
                kind: Kind::LitChar,
                span: Span {
                    start,
                    end: self.cursor,
                },
            }))
        }
    }

    /// Consume a string literal.
    fn next_lit_byte<I>(&mut self, it: &mut I, start: usize) -> Result<Option<Token>, ParseError>
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        self.cursor = loop {
            let (n, c) = match it.clone().next() {
                Some(c) => c,
                None => {
                    return Err(ParseError::ExpectedByteClose {
                        span: Span {
                            start,
                            end: self.source.len(),
                        },
                    })
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
                    return Err(ParseError::UnterminatedByteLit {
                        span: Span {
                            start,
                            end: self.cursor + n,
                        },
                    });
                }
                _ => {
                    it.next();
                }
            }
        };

        Ok(Some(Token {
            kind: Kind::LitByte,
            span: Span {
                start,
                end: self.cursor,
            },
        }))
    }

    /// Consume a string literal.
    fn next_lit_str<I>(&mut self, it: &mut I, start: usize) -> Result<Option<Token>, ParseError>
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
                            return Err(ParseError::ExpectedStringEscape {
                                span: Span {
                                    start,
                                    end: self.source.len(),
                                },
                            });
                        }
                    },
                    _ => continue,
                },
                None => {
                    return Err(ParseError::UnterminatedStrLit {
                        span: Span {
                            start,
                            end: self.source.len(),
                        },
                    })
                }
            };
        };

        Ok(Some(Token {
            kind: Kind::LitStr { escaped },
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
    ) -> Result<Option<Token>, ParseError>
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
                            return Err(ParseError::ExpectedStringEscape {
                                span: Span {
                                    start,
                                    end: self.source.len(),
                                },
                            });
                        }
                    },
                    _ => continue,
                },
                None => {
                    return Err(ParseError::UnterminatedStrLit {
                        span: Span {
                            start,
                            end: self.source.len(),
                        },
                    })
                }
            };
        };

        Ok(Some(Token {
            kind: Kind::LitByteStr { escaped },
            span: Span {
                start,
                end: self.cursor,
            },
        }))
    }

    /// Consume a string literal.
    fn next_template<I>(&mut self, it: &mut I, start: usize) -> Result<Option<Token>, ParseError>
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
                            return Err(ParseError::ExpectedTemplateClose {
                                span: Span {
                                    start,
                                    end: self.source.len(),
                                },
                            });
                        }
                    },
                    _ => continue,
                },
                None => {
                    return Err(ParseError::ExpectedTemplateClose {
                        span: Span {
                            start,
                            end: self.source.len(),
                        },
                    })
                }
            };
        };

        Ok(Some(Token {
            kind: Kind::LitTemplate { escaped },
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
    pub fn next(&mut self) -> Result<Option<Token>, ParseError> {
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
                            break Kind::AddAssign;
                        }
                        ('-', '=') => {
                            it.next();
                            break Kind::SubAssign;
                        }
                        ('*', '=') => {
                            it.next();
                            break Kind::MulAssign;
                        }
                        ('/', '=') => {
                            it.next();
                            break Kind::DivAssign;
                        }
                        ('/', '/') => {
                            self.consume_line(&mut it);
                            continue 'outer;
                        }
                        (':', ':') => {
                            it.next();
                            break Kind::Scope;
                        }
                        ('<', '=') => {
                            it.next();
                            break Kind::Lte;
                        }
                        ('>', '=') => {
                            it.next();
                            break Kind::Gte;
                        }
                        ('=', '=') => {
                            it.next();
                            break Kind::EqEq;
                        }
                        ('!', '=') => {
                            it.next();
                            break Kind::Neq;
                        }
                        ('&', '&') => {
                            it.next();
                            break Kind::And;
                        }
                        ('|', '|') => {
                            it.next();
                            break Kind::Or;
                        }
                        ('.', '.') => {
                            it.next();
                            break Kind::DotDot;
                        }
                        ('=', '>') => {
                            it.next();
                            break Kind::Rocket;
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
                    '(' => Kind::Open(Delimiter::Parenthesis),
                    ')' => Kind::Close(Delimiter::Parenthesis),
                    '{' => Kind::Open(Delimiter::Brace),
                    '}' => Kind::Close(Delimiter::Brace),
                    '[' => Kind::Open(Delimiter::Bracket),
                    ']' => Kind::Close(Delimiter::Bracket),
                    '_' => Kind::Underscore,
                    ',' => Kind::Comma,
                    ':' => Kind::Colon,
                    '#' => Kind::Hash,
                    '.' => Kind::Dot,
                    ';' => Kind::SemiColon,
                    '=' => Kind::Eq,
                    '+' => Kind::Add,
                    '-' => Kind::Sub,
                    '/' => Kind::Div,
                    '*' => Kind::Mul,
                    '&' => Kind::Ampersand,
                    '>' => Kind::Gt,
                    '<' => Kind::Lt,
                    '!' => Kind::Bang,
                    '?' => Kind::Try,
                    '|' => Kind::Pipe,
                    '%' => Kind::Rem,
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

                        return Err(ParseError::UnexpectedChar { span, c });
                    }
                };
            };

            self.cursor = self.end_span(&it);

            return Ok(Some(Token {
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
    use crate::token::{Delimiter, Kind, LitNumber, Token};
    use runestick::unit::Span;

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
            Token {
                span: Span::new(0, 3),
                kind: Kind::LitChar,
            }
        };

        test_lexer! {
            "'\\u{abcd}'",
            Token {
                span: Span::new(0, 10),
                kind: Kind::LitChar,
            }
        };
    }

    #[test]
    fn test_label() {
        test_lexer! {
            "'asdf 'a' \"foo bar\"",
            Token {
                span: Span::new(0, 5),
                kind: Kind::Label,
            },
            Token {
                span: Span::new(6, 9),
                kind: Kind::LitChar,
            },
            Token {
                span: Span::new(10, 19),
                kind: Kind::LitStr {
                    escaped: false,
                },
            }
        };
    }

    #[test]
    fn test_operators() {
        test_lexer! {
            "+ += - -= * *= / /=",
            Token {
                span: Span::new(0, 1),
                kind: Kind::Add,
            },
            Token {
                span: Span::new(2, 4),
                kind: Kind::AddAssign,
            },
            Token {
                span: Span::new(5, 6),
                kind: Kind::Sub,
            },
            Token {
                span: Span::new(7, 9),
                kind: Kind::SubAssign,
            },
            Token {
                span: Span::new(10, 11),
                kind: Kind::Mul,
            },
            Token {
                span: Span::new(12, 14),
                kind: Kind::MulAssign,
            },
            Token {
                span: Span::new(15, 16),
                kind: Kind::Div,
            },
            Token {
                span: Span::new(17, 19),
                kind: Kind::DivAssign,
            }
        };
    }

    #[test]
    fn test_idents() {
        test_lexer! {
            "a.checked_div(10)",
            Token {
                span: Span::new(0, 1),
                kind: Kind::Ident,
            },
            Token {
                span: Span::new(1, 2),
                kind: Kind::Dot,
            },
            Token {
                span: Span::new(2, 13),
                kind: Kind::Ident,
            },
            Token {
                span: Span::new(13, 14),
                kind: Kind::Open(Delimiter::Parenthesis),
            },
            Token {
                span: Span::new(14, 16),
                kind: Kind::LitNumber {
                    is_fractional: false,
                    is_negative: false,
                    number: LitNumber::Decimal,
                },
            },
            Token {
                span: Span::new(16, 17),
                kind: Kind::Close(Delimiter::Parenthesis),
            },
        };
    }

    #[test]
    fn test_template_literals() {
        test_lexer! {
            "`foo {bar} \\` baz`",
            Token {
                span: Span::new(0, 18),
                kind: Kind::LitTemplate { escaped: true },
            },
        };
    }

    #[test]
    fn test_literals() {
        test_lexer! {
            r#"b"hello world""#,
            Token {
                span: Span::new(0, 14),
                kind: Kind::LitByteStr {
                    escaped: false,
                },
            },
        };

        test_lexer! {
            "b'\\\\''",
            Token {
                span: Span::new(0, 6),
                kind: Kind::LitByte,
            },
        };

        test_lexer! {
            "'label 'a' b'a'",
            Token {
                span: Span::new(0, 6),
                kind: Kind::Label,
            },
            Token {
                span: Span::new(7, 10),
                kind: Kind::LitChar,
            },
            Token {
                span: Span::new(11, 15),
                kind: Kind::LitByte,
            },
        };

        test_lexer! {
            "b'a'",
            Token {
                span: Span::new(0, 4),
                kind: Kind::LitByte,
            },
        };

        test_lexer! {
            "b'\\n'",
            Token {
                span: Span::new(0, 5),
                kind: Kind::LitByte,
            },
        };
    }
}
