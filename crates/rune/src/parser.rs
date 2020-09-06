use crate::ast::Token;
use crate::error::ParseError;
use crate::lexer::Lexer;
use crate::traits::{Parse, Peek};

/// Parser for the rune language.
///
/// # Examples
///
/// ```rust
/// use rune::{ast, Parser};
///
/// let mut parser = Parser::new("fn foo() {}");
/// parser.parse::<ast::DeclFn>().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct Parser<'a> {
    pub(crate) lexer: Lexer<'a>,
    p1: Result<Option<Token>, ParseError>,
    p2: Result<Option<Token>, ParseError>,
    p3: Result<Option<Token>, ParseError>,
}

impl<'a> Parser<'a> {
    /// Construct a new parser around the given source.
    pub fn new(source: &'a str) -> Self {
        Self::new_with_start(source, 0)
    }

    /// Construct a new parser around the given source.
    pub fn new_with_start(source: &'a str, start: usize) -> Self {
        let mut lexer = Lexer::new_with_start(source, start);

        let p1 = lexer.next();
        let p2 = lexer.next();
        let p3 = lexer.next();

        Self { lexer, p1, p2, p3 }
    }

    /// Parse a specific item from the parser.
    pub fn parse<T>(&mut self) -> Result<T, ParseError>
    where
        T: Parse,
    {
        T::parse(self)
    }

    /// Peek for the given token.
    pub fn peek<T>(&self) -> Result<bool, ParseError>
    where
        T: Peek,
    {
        Ok(T::peek(self.p1?, self.p2?))
    }

    /// Peek for the given token.
    pub fn peek2<T>(&self) -> Result<bool, ParseError>
    where
        T: Peek,
    {
        Ok(T::peek(self.p2?, self.p3?))
    }

    /// Peek the current token.
    pub(crate) fn token_peek(&mut self) -> Result<Option<Token>, ParseError> {
        self.p1
    }

    /// Peek the next two tokens.
    pub(crate) fn token_peek_pair(&mut self) -> Result<Option<(Token, Option<Token>)>, ParseError> {
        Ok(match self.p1? {
            Some(p1) => Some((p1, self.p2?)),
            None => None,
        })
    }

    /// Consume the next token from the lexer.
    pub(crate) fn token_next(&mut self) -> Result<Token, ParseError> {
        let token = std::mem::replace(&mut self.p3, self.lexer.next());
        let token = std::mem::replace(&mut self.p2, token);
        let token = std::mem::replace(&mut self.p1, token);

        match token? {
            Some(token) => Ok(token),
            None => Err(ParseError::UnexpectedEof {
                span: self.lexer.end(),
            }),
        }
    }

    /// Peek the current token from the lexer but treat a missing token as an
    /// unexpected end-of-file.
    pub(crate) fn token_peek_eof(&mut self) -> Result<Token, ParseError> {
        match self.p1? {
            Some(token) => Ok(token),
            None => Err(ParseError::UnexpectedEof {
                span: self.lexer.end(),
            }),
        }
    }
}
