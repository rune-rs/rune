use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A literal expression.
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
pub struct ExprLit {
    /// Attributes associated with the literal expression.
    #[rune(iter, attributes)]
    pub attributes: Vec<ast::Attribute>,
    /// The literal in the expression.
    pub lit: ast::Lit,
}

impl ExprLit {
    /// Test if the literal expression is constant.
    pub fn is_const(&self) -> bool {
        self.lit.is_const()
    }
}
