use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprLit>("42");
    rt::<ast::ExprLit>("\"test\"");
    rt::<ast::ExprLit>("#[attr] 42");
}

/// A literal expression. With the addition of being able to receive attributes,
/// this is identical to [ast::Lit].
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
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
