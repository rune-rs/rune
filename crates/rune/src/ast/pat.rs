use crate::ast::{Ident, LitChar, LitNumber, LitStr, LitUnit, PatArray, PatObject, Underscore};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::token::{Delimiter, Kind, Token};
use crate::traits::{Parse, Peek};
use stk::unit::Span;

/// A pattern match.
#[derive(Debug, Clone)]
pub enum Pat {
    /// An ignored binding `_`.
    PatIgnore(Underscore),
    /// A variable binding `n`.
    PatBinding(Ident),
    /// A literal unit.
    PatUnit(LitUnit),
    /// A literal character.
    PatChar(LitChar),
    /// A literal number.
    PatNumber(LitNumber),
    /// A literal string.
    PatString(LitStr),
    /// An array pattern.
    PatArray(PatArray),
    /// An object pattern.
    PatObject(PatObject),
}

impl Pat {
    /// Get the span of the pattern.
    pub fn span(&self) -> Span {
        match self {
            Self::PatUnit(expr) => expr.span(),
            Self::PatChar(expr) => expr.span(),
            Self::PatNumber(expr) => expr.span(),
            Self::PatString(expr) => expr.span(),
            Self::PatBinding(expr) => expr.span(),
            Self::PatIgnore(expr) => expr.span(),
            Self::PatArray(expr) => expr.span(),
            Self::PatObject(expr) => expr.span(),
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
            } => Self::PatUnit(parser.parse()?),
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
