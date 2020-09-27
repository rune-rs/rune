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
