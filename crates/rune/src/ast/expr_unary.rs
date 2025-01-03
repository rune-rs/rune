use core::fmt;

use crate::ast::prelude::*;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
    rt::<ast::ExprUnary>("!0");
    rt::<ast::ExprUnary>("*foo");
    rt::<ast::ExprUnary>("&foo");
    rt::<ast::ExprUnary>(
        "&Foo {
        a: 42,
    }",
    );

    rt::<ast::UnOp>("!");
    rt::<ast::UnOp>("-");
    rt::<ast::UnOp>("&");
    rt::<ast::UnOp>("*");
}

/// A unary expression.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprUnary {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The operation to apply.
    pub op: UnOp,
    /// The expression of the operation.
    pub expr: Box<ast::Expr>,
}

impl ExprUnary {
    /// Parse the uniary expression with the given meta and configuration.
    pub(crate) fn parse_with_meta(
        p: &mut Parser,
        attributes: Vec<ast::Attribute>,
        eager_brace: ast::expr::EagerBrace,
    ) -> Result<Self> {
        Ok(Self {
            attributes,
            op: p.parse()?,
            expr: Box::try_new(ast::Expr::parse_with(
                p,
                eager_brace,
                ast::expr::NOT_EAGER_BINARY,
                ast::expr::CALLABLE,
            )?)?,
        })
    }
}

expr_parse!(Unary, ExprUnary, "try expression");

/// A unary operation.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, ToTokens, Spanned)]
#[try_clone(copy)]
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

impl ToAst for UnOp {
    fn to_ast(span: Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            K![!] => Ok(Self::Not(ast::Bang { span })),
            K![-] => Ok(Self::Neg(ast::Dash { span })),
            K![&] => Ok(Self::BorrowRef(ast::Amp { span })),
            K![*] => Ok(Self::Deref(ast::Star { span })),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                "unary operator, like `!` or `-`",
            )),
        }
    }

    #[inline]
    fn matches(kind: &ast::Kind) -> bool {
        matches!(kind, K![!] | K![-] | K![&] | K![*])
    }

    #[inline]
    fn into_expectation() -> Expectation {
        Expectation::Description("a unary operation")
    }
}

impl Parse for UnOp {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        let token = p.next()?;
        Self::to_ast(token.span, token.kind)
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
