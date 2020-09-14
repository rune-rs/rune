use crate::ast;
use crate::{Parse, ParseError, Parser, Peek};
use runestick::Span;

/// A pattern match.
#[derive(Debug, Clone)]
pub enum Pat {
    /// An ignored binding `_`.
    PatIgnore(ast::Underscore),
    /// A variable binding `n`.
    PatPath(ast::PatPath),
    /// A literal unit.
    PatUnit(ast::LitUnit),
    /// A literal byte.
    PatByte(ast::LitByte),
    /// A literal character.
    PatChar(ast::LitChar),
    /// A literal number.
    PatNumber(ast::LitNumber),
    /// A literal string.
    PatString(ast::LitStr),
    /// A vector pattern.
    PatVec(ast::PatVec),
    /// A tuple pattern.
    PatTuple(ast::PatTuple),
    /// An object pattern.
    PatObject(ast::PatObject),
}

into_tokens_enum!(Pat {
    PatIgnore,
    PatPath,
    PatUnit,
    PatByte,
    PatChar,
    PatNumber,
    PatString,
    PatVec,
    PatTuple,
    PatObject
});

impl Pat {
    /// Get the span of the pattern.
    pub fn span(&self) -> Span {
        match self {
            Self::PatUnit(pat) => pat.span(),
            Self::PatByte(pat) => pat.span(),
            Self::PatChar(pat) => pat.span(),
            Self::PatNumber(pat) => pat.span(),
            Self::PatString(pat) => pat.span(),
            Self::PatPath(pat) => pat.span(),
            Self::PatIgnore(pat) => pat.span(),
            Self::PatVec(pat) => pat.span(),
            Self::PatTuple(pat) => pat.span(),
            Self::PatObject(pat) => pat.span(),
        }
    }

    /// Parse a pattern with a starting identifier.
    pub fn parse_ident(parser: &mut Parser) -> Result<Self, ParseError> {
        let path: ast::Path = parser.parse()?;

        let t = match parser.token_peek()? {
            Some(t) => t,
            None => return Ok(Self::PatPath(ast::PatPath { path })),
        };

        Ok(match t.kind {
            ast::Kind::Open(ast::Delimiter::Parenthesis) => {
                Self::PatTuple(ast::PatTuple::parse_with_path(parser, Some(path))?)
            }
            ast::Kind::Open(ast::Delimiter::Brace) => {
                let ident = ast::LitObjectIdent::Named(path);

                Self::PatObject(ast::PatObject::parse_with_ident(parser, ident)?)
            }
            _ => Self::PatPath(ast::PatPath { path }),
        })
    }
}

/// Parsing a block expression.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::Pat>("()").unwrap();
/// parse_all::<ast::Pat>("1").unwrap();
/// parse_all::<ast::Pat>("'a'").unwrap();
/// parse_all::<ast::Pat>("\"hello world\"").unwrap();
/// parse_all::<ast::Pat>("var").unwrap();
/// parse_all::<ast::Pat>("_").unwrap();
/// parse_all::<ast::Pat>("Foo(n)").unwrap();
/// ```
impl Parse for Pat {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            ast::Kind::Open(ast::Delimiter::Parenthesis) => {
                if parser.peek::<ast::LitUnit>()? {
                    Self::PatUnit(parser.parse()?)
                } else {
                    Self::PatTuple(parser.parse()?)
                }
            }
            ast::Kind::Open(ast::Delimiter::Bracket) => Self::PatVec(parser.parse()?),
            ast::Kind::Pound => Self::PatObject(parser.parse()?),
            ast::Kind::LitByte { .. } => Self::PatByte(parser.parse()?),
            ast::Kind::LitChar { .. } => Self::PatChar(parser.parse()?),
            ast::Kind::LitNumber { .. } => Self::PatNumber(parser.parse()?),
            ast::Kind::LitStr { .. } => Self::PatString(parser.parse()?),
            ast::Kind::Underscore => Self::PatIgnore(parser.parse()?),
            ast::Kind::Ident(..) => Self::parse_ident(parser)?,
            _ => {
                return Err(ParseError::ExpectedPatError {
                    span: token.span,
                    actual: token.kind,
                })
            }
        })
    }
}

impl Peek for Pat {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        match t1.kind {
            ast::Kind::Open(ast::Delimiter::Parenthesis) => true,
            ast::Kind::Open(ast::Delimiter::Bracket) => true,
            ast::Kind::Pound => true,
            ast::Kind::LitByte { .. } => true,
            ast::Kind::LitChar { .. } => true,
            ast::Kind::LitNumber { .. } => true,
            ast::Kind::LitStr { .. } => true,
            ast::Kind::Underscore => true,
            ast::Kind::Ident(..) => true,
            _ => false,
        }
    }
}
