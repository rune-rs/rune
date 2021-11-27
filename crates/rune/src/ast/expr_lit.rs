use crate::ast::prelude::*;

/// A literal expression. With the addition of being able to receive attributes,
/// this is identical to [ast::Lit].
///
/// # Examples
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::ExprLit>("42");
/// testing::roundtrip::<ast::ExprLit>("\"test\"");
/// testing::roundtrip::<ast::ExprLit>("#[attr] 42");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ExprLit {
    /// Attributes associated with the literal expression.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The literal in the expression.
    pub lit: ast::Lit,
}

expr_parse!(Lit, ExprLit, "literal expression");
