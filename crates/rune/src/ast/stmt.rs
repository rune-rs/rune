use crate::ast;
use crate::{IntoTokens, Spanned};
use runestick::Span;

/// A statement within a block.
#[derive(Debug, Clone)]
pub enum Stmt {
    /// A declaration.
    Item(ast::Item),
    /// An expression.
    Expr(ast::Expr),
    /// An expression followed by a semicolon.
    Semi(ast::Expr, ast::SemiColon),
}

impl Spanned for Stmt {
    fn span(&self) -> Span {
        match self {
            Self::Item(decl) => decl.span(),
            Self::Expr(expr) => expr.span(),
            Self::Semi(expr, semi) => expr.span().join(semi.span()),
        }
    }
}

impl IntoTokens for Stmt {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        match self {
            Self::Item(decl) => decl.into_tokens(context, stream),
            Self::Expr(expr) => expr.into_tokens(context, stream),
            Self::Semi(expr, semi) => {
                expr.into_tokens(context, stream);
                semi.into_tokens(context, stream);
            }
        }
    }
}
