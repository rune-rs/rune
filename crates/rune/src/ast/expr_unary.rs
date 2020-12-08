use crate::ast;
use crate::ast::expr::EagerBrace;
use crate::{ParseError, Parser, Spanned, ToTokens};
use runestick::Span;
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
pub struct ExprUnary {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// Token associated with operator.
    pub op_token: ast::Token,
    /// The expression of the operation.
    pub expr: ast::Expr,
    /// The operation to apply.
    #[rune(skip)]
    pub op: UnOp,
}

impl ExprUnary {
    /// Get the span of the op.
    pub fn op_span(&self) -> Span {
        self.op_token.span()
    }

    /// Parse the uniary expression with the given meta and configuration.
    pub(crate) fn parse_with_meta(
        parser: &mut Parser,
        attributes: Vec<ast::Attribute>,
        eager_brace: EagerBrace,
    ) -> Result<Self, ParseError> {
        let op_token = parser.next()?;
        let op = UnOp::from_token(op_token)?;

        Ok(Self {
            attributes,
            op_token,
            expr: ast::Expr::parse_with(
                parser,
                eager_brace,
                ast::expr::EagerBinary(false),
                ast::expr::Callable(true),
            )?,
            op,
        })
    }
}

expr_parse!(Unary, ExprUnary, "try expression");

/// A unary operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// Not `!<thing>`.
    Not,
    /// Negation `-<thing>`.
    Neg,
    /// Reference `&<thing>`.
    BorrowRef,
    /// Dereference `*<thing>`.
    Deref,
}

impl UnOp {
    /// Convert a unary operator from a token.
    pub fn from_token(t: ast::Token) -> Result<Self, ParseError> {
        match t.kind {
            K![!] => Ok(Self::Not),
            K![-] => Ok(Self::Neg),
            K![&] => Ok(Self::BorrowRef),
            K![*] => Ok(Self::Deref),
            _ => Err(ParseError::expected(&t, "unary operator, like `!` or `-`")),
        }
    }
}

impl fmt::Display for UnOp {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Not => write!(fmt, "!")?,
            Self::Neg => write!(fmt, "-")?,
            Self::BorrowRef => write!(fmt, "&")?,
            Self::Deref => write!(fmt, "*")?,
        }

        Ok(())
    }
}
