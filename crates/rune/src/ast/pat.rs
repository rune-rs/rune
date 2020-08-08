use crate::ast;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::token::{Delimiter, Kind, Token};
use crate::traits::{Parse, Peek};
use runestick::unit::Span;

/// A pattern match.
#[derive(Debug, Clone)]
pub enum Pat {
    /// An ignored binding `_`.
    PatIgnore(ast::Underscore),
    /// A variable binding `n`.
    PatBinding(ast::Ident),
    /// A literal unit.
    PatUnit(ast::LitUnit),
    /// A literal character.
    PatChar(ast::LitChar),
    /// A literal number.
    PatNumber(ast::LitNumber),
    /// A literal string.
    PatString(ast::LitStr),
    /// An array pattern.
    PatArray(ast::PatArray),
    /// A tuple pattern.
    PatTuple(ast::PatTuple),
    /// An object pattern.
    PatObject(ast::PatObject),
}

impl Pat {
    /// Get the span of the pattern.
    pub fn span(&self) -> Span {
        match self {
            Self::PatUnit(pat) => pat.span(),
            Self::PatChar(pat) => pat.span(),
            Self::PatNumber(pat) => pat.span(),
            Self::PatString(pat) => pat.span(),
            Self::PatBinding(pat) => pat.span(),
            Self::PatIgnore(pat) => pat.span(),
            Self::PatArray(pat) => pat.span(),
            Self::PatTuple(pat) => pat.span(),
            Self::PatObject(pat) => pat.span(),
        }
    }
}

/// Parsing a block expression.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() {
/// parse_all::<ast::Pat>("()").unwrap();
/// parse_all::<ast::Pat>("1").unwrap();
/// parse_all::<ast::Pat>("'a'").unwrap();
/// parse_all::<ast::Pat>("\"hello world\"").unwrap();
/// parse_all::<ast::Pat>("var").unwrap();
/// parse_all::<ast::Pat>("_").unwrap();
/// # }
/// ```
impl Parse for Pat {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            Kind::Open {
                delimiter: Delimiter::Parenthesis,
            } => {
                if parser.peek::<ast::LitUnit>()? {
                    Self::PatUnit(parser.parse()?)
                } else {
                    Self::PatTuple(parser.parse()?)
                }
            }
            Kind::Open {
                delimiter: Delimiter::Bracket,
            } => Self::PatArray(parser.parse()?),
            Kind::StartObject => Self::PatObject(parser.parse()?),
            Kind::LitChar { .. } => Self::PatChar(parser.parse()?),
            Kind::LitNumber { .. } => Self::PatNumber(parser.parse()?),
            Kind::LitStr { .. } => Self::PatString(parser.parse()?),
            Kind::Underscore => Self::PatIgnore(parser.parse()?),
            Kind::Ident => Self::PatBinding(parser.parse()?),
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
    fn peek(t1: Option<Token>, _: Option<Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        match t1.kind {
            Kind::Open {
                delimiter: Delimiter::Parenthesis,
            } => true,
            Kind::Open {
                delimiter: Delimiter::Bracket,
            } => true,
            Kind::StartObject => true,
            Kind::LitChar { .. } => true,
            Kind::LitNumber { .. } => true,
            Kind::LitStr { .. } => true,
            Kind::Underscore => true,
            Kind::Ident => true,
            _ => false,
        }
    }
}
