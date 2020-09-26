use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A block of expressions.
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
pub struct ExprBlock {
    /// The attributes for the block.
    #[rune(iter, attributes)]
    pub attributes: Vec<ast::Attribute>,
    /// The close brace.
    pub block: ast::Block,
}

impl ExprBlock {
    /// Test if the block expression doesn't produce a value.
    pub fn produces_nothing(&self) -> bool {
        self.block.produces_nothing()
    }
}
