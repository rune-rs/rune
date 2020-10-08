use crate::ast;
use crate::{Spanned, ToTokens};
use runestick::Span;

/// Helper to force an expression to have a specific semi-colon policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceSemi {
    /// The span of the whole wrapping expression.
    pub span: Span,
    /// Whether or not the expressions needs a semi.
    pub needs_semi: bool,
    /// The expression to override the policy for.
    pub expr: ast::Expr,
}

impl Spanned for ForceSemi {
    fn span(&self) -> Span {
        self.span
    }
}

impl ToTokens for ForceSemi {
    fn to_tokens(&self, context: &crate::MacroContext, stream: &mut crate::TokenStream) {
        self.expr.to_tokens(context, stream)
    }
}
