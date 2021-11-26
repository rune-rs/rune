use crate::ast::prelude::*;
use std::fmt;

/// A unary expression.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprUnary>("!0");
/// testing::roundtrip::<ast::ExprUnary>("*foo");
/// testing::roundtrip::<ast::ExprUnary>("&foo");
/// testing::roundtrip::<ast::ExprUnary>("&Foo {
///     a: 42,
/// }");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprUnary {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The operation to apply.
    pub op: UnOp,
    /// The expression of the operation.
    pub expr: ast::Expr,
}

impl ExprUnary {
    /// Parse the uniary expression with the given meta and configuration.
    pub(crate) fn parse_with_meta(
        p: &mut Parser,
        attributes: Vec<ast::Attribute>,
        eager_brace: ast::expr::EagerBrace,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            op: p.parse()?,
            expr: ast::Expr::parse_with(
                p,
                eager_brace,
                ast::expr::NOT_EAGER_BINARY,
                ast::expr::CALLABLE,
            )?,
        })
    }
}

expr_parse!(Unary, ExprUnary, "try expression");

/// A unary operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ToTokens, Spanned)]
pub enum UnOp {
    /// Not `!<thing>`.
    Not(ast::Bang),
    /// Negation `-<thing>`.
    Neg(ast::Dash),
    /// Reference `&<thing>`.
    BorrowRef(ast::Amp),
    /// Dereference `*<thing>`.
    Deref(ast::Star),
}

/// A unary operator.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::UnOp>("!");
/// testing::roundtrip::<ast::UnOp>("-");
/// testing::roundtrip::<ast::UnOp>("&");
/// testing::roundtrip::<ast::UnOp>("*");
/// ```
impl Parse for UnOp {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        let token = p.next()?;

        match token.kind {
            K![!] => Ok(Self::Not(ast::Bang { span: token.span })),
            K![-] => Ok(Self::Neg(ast::Dash { span: token.span })),
            K![&] => Ok(Self::BorrowRef(ast::Amp { span: token.span })),
            K![*] => Ok(Self::Deref(ast::Star { span: token.span })),
            _ => Err(ParseError::expected(
                token,
                "unary operator, like `!` or `-`",
            )),
        }
    }
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Not(..) => write!(f, "!")?,
            Self::Neg(..) => write!(f, "-")?,
            Self::BorrowRef(..) => write!(f, "&")?,
            Self::Deref(..) => write!(f, "*")?,
        }

        Ok(())
    }
}
