use crate::error::{ParseError, Result};
use crate::token::{Delimiter, Kind, NumberLiteral, Span, Token};

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
            .unwrap_or(self.source.len())
    }

    fn next_ident<I>(&mut self, it: &mut I, start: usize) -> Result<Option<Token>, ParseError>
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        self.cursor = loop {
            break match it.clone().next() {
                Some((n, c)) => match c {
                    'a'..='z' | 'A'..='Z' | '_' => {
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
            "fn" => Kind::Fn,
            "let" => Kind::Let,
            "if" => Kind::If,
            "else" => Kind::Else,
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
        start: usize,
    ) -> Result<Option<Token>, ParseError>
    where
        I: Clone + Iterator<Item = (usize, char)>,
    {
        let number = match it.clone().next() {
            Some((_, c)) => match c {
                'x' => NumberLiteral::Hex,
                'b' => NumberLiteral::Binary,
                'o' => NumberLiteral::Octal,
                _ => NumberLiteral::Decimal,
            },
            _ => NumberLiteral::Decimal,
        };

        self.cursor = loop {
            break match it.next() {
                Some((n, c)) => {
                    if char::is_alphanumeric(c) {
                        continue;
                    } else {
                        self.cursor + n
                    }
                }
                None => self.source.len(),
            };
        };

        return Ok(Some(Token {
            kind: Kind::NumberLiteral { number },
            span: Span {
                start,
                end: self.cursor,
            },
        }));
    }

    /// Consume a string literal.
    fn next_string_literal<I>(
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
                    return Err(ParseError::ExpectedStringClose {
                        span: Span {
                            start,
                            end: self.source.len(),
                        },
                    })
                }
            };
        };

        return Ok(Some(Token {
            kind: Kind::StringLiteral { escaped },
            span: Span {
                start,
                end: self.cursor,
            },
        }));
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
    pub fn next(&mut self) -> Result<Option<Token>, ParseError> {
        let mut it = self.source[self.cursor..].char_indices();

        'outer: while let Some((start, c)) = it.next() {
            let start = self.cursor + start;

            if char::is_whitespace(c) {
                continue;
            }

            let kind = loop {
                match (c, it.clone().next().map(|(_, c)| c)) {
                    ('/', Some('/')) => {
                        self.consume_line(&mut it);
                        continue 'outer;
                    }
                    ('<', Some('=')) => {
                        it.next();
                        break Kind::Lte;
                    }
                    ('>', Some('=')) => {
                        it.next();
                        break Kind::Gte;
                    }
                    ('=', Some('=')) => {
                        it.next();
                        break Kind::EqEq;
                    }
                    _ => (),
                }

                break match c {
                    '(' => Kind::Open {
                        delimiter: Delimiter::Parenthesis,
                    },
                    ')' => Kind::Close {
                        delimiter: Delimiter::Parenthesis,
                    },
                    '{' => Kind::Open {
                        delimiter: Delimiter::Brace,
                    },
                    '}' => Kind::Close {
                        delimiter: Delimiter::Brace,
                    },
                    '[' => Kind::Open {
                        delimiter: Delimiter::Bracket,
                    },
                    ']' => Kind::Close {
                        delimiter: Delimiter::Bracket,
                    },
                    ',' => Kind::Comma,
                    ';' => Kind::SemiColon,
                    '=' => Kind::Eq,
                    '+' => Kind::Plus,
                    '-' => Kind::Minus,
                    '/' => Kind::Slash,
                    '*' => Kind::Star,
                    '>' => Kind::Gt,
                    '<' => Kind::Lt,
                    'a'..='z' | 'A'..='Z' => {
                        return self.next_ident(&mut it, start);
                    }
                    '0'..='9' => {
                        return self.next_number_literal(&mut it, start);
                    }
                    '"' => {
                        return self.next_string_literal(&mut it, start);
                    }
                    _ => {
                        let span = Span {
                            start,
                            end: self.end_span(&mut it),
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
