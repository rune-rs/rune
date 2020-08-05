use crate::error::{ParseError, Result};
use crate::token::{Delimiter, Kind, LitNumber, Token};
use st::unit::Span;

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
            "fn" => Kind::Fn,
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
            "break" => Kind::Break,
            "return" => Kind::Return,
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
        let mut is_fractional = false;

        let number = match it.clone().next() {
            Some((_, c)) => match c {
                'x' => LitNumber::Hex,
                'b' => LitNumber::Binary,
                'o' => LitNumber::Octal,
                _ => LitNumber::Decimal,
            },
            _ => LitNumber::Decimal,
        };

        self.cursor = loop {
            break match it.next() {
                Some((n, c)) => match c {
                    c if char::is_alphanumeric(c) => continue,
                    '.' => {
                        is_fractional = true;
                        continue;
                    }
                    '-' => continue,
                    _ => self.cursor + n,
                },
                None => self.source.len(),
            };
        };

        return Ok(Some(Token {
            kind: Kind::LitNumber {
                is_fractional,
                number,
            },
            span: Span {
                start,
                end: self.cursor,
            },
        }));
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
        let mut is_char_literal = false;

        self.cursor = loop {
            break match it.clone().next() {
                Some((n, c)) => match c {
                    'a'..='z' | 'A'..='Z' | '_' => {
                        it.next();
                        continue;
                    }
                    '\\' => {
                        is_char_literal = true;
                        it.next();
                        it.next();
                        continue;
                    }
                    '\'' => {
                        is_char_literal = true;
                        it.next();
                        self.end_span(it)
                    }
                    _ => self.cursor + n,
                },
                None => self.source.len(),
            };
        };

        if is_char_literal {
            return Ok(Some(Token {
                kind: Kind::LitChar,
                span: Span {
                    start,
                    end: self.cursor,
                },
            }));
        }

        Ok(Some(Token {
            kind: Kind::Label,
            span: Span {
                start,
                end: self.cursor,
            },
        }))
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
            kind: Kind::LitStr { escaped },
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
                        ('#', '{') => {
                            it.next();
                            break Kind::StartObject;
                        }
                        ('.', '.') => {
                            it.next();
                            break Kind::DotDot;
                        }
                        ('=', '>') => {
                            it.next();
                            break Kind::Rocket;
                        }
                        ('-', '0'..='9') => {
                            return self.next_number_literal(&mut it, start);
                        }
                        _ => (),
                    }
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
                    '_' => Kind::Underscore,
                    ',' => Kind::Comma,
                    ':' => Kind::Colon,
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
                    '!' => Kind::Not,
                    'a'..='z' | 'A'..='Z' => {
                        return self.next_ident(&mut it, start);
                    }
                    '0'..='9' => {
                        return self.next_number_literal(&mut it, start);
                    }
                    '"' => {
                        return self.next_string_literal(&mut it, start);
                    }
                    '\'' => {
                        return self.next_char_or_label(&mut it, start);
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

#[cfg(test)]
mod tests {
    use super::Lexer;
    use crate::token::{Kind, Token};
    use st::unit::Span;

    macro_rules! test_lexer {
        ($source:expr $(, $pat:expr)*) => {{
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
}
