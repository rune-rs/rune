use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};
use std::fmt;

/// A unary expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprUnary {
    /// Token associated with operator.
    pub token: ast::Token,
    /// The expression of the operation.
    pub expr: Box<ast::Expr>,
    /// The operation to apply.
    #[rune(skip)]
    pub op: UnaryOp,
}

/// Parse a unary statement.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprUnary>("!0");
/// testing::roundtrip::<ast::ExprUnary>("*foo");
/// testing::roundtrip::<ast::ExprUnary>("&foo");
/// ```
impl Parse for ExprUnary {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_next()?;
        let path = parser.parse::<Option<ast::Path>>()?;

        Ok(Self {
            op: UnaryOp::from_token(token)?,
            token,
            expr: Box::new(ast::Expr::parse_with_meta(parser, &mut vec![], path)?),
        })
    }
}

/// A unary operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// Not `!<thing>`.
    Not,
    /// Reference `&<thing>`.
    BorrowRef,
    /// Dereference `*<thing>`.
    Deref,
}

impl UnaryOp {
    /// Convert a unary operator from a token.
    pub fn from_token(token: ast::Token) -> Result<Self, ParseError> {
        match token.kind {
            ast::Kind::Bang => Ok(Self::Not),
            ast::Kind::Amp => Ok(Self::BorrowRef),
            ast::Kind::Star => Ok(Self::Deref),
            _ => Err(ParseError::expected(token, "unary operator `!`")),
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Not => write!(fmt, "!")?,
            Self::BorrowRef => write!(fmt, "&")?,
            Self::Deref => write!(fmt, "*")?,
        }

        Ok(())
    }
}
