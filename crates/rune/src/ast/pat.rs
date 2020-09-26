use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Spanned, ToTokens};

/// A pattern match.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
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

impl Pat {
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
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Pat>("()");
/// testing::roundtrip::<ast::Pat>("1");
/// testing::roundtrip::<ast::Pat>("'a'");
/// testing::roundtrip::<ast::Pat>("\"hello world\"");
/// testing::roundtrip::<ast::Pat>("var");
/// testing::roundtrip::<ast::Pat>("_");
/// testing::roundtrip::<ast::Pat>("Foo(n)");
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
                return Err(ParseError::expected(token, "pattern"));
            }
        })
    }
}

impl Peek for Pat {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        match peek!(t1).kind {
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
