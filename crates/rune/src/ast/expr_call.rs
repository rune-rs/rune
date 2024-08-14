use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    rt::<ast::ExprCall>("test()");
    rt::<ast::ExprCall>("(foo::bar)()");
}

/// A call expression.
///
/// * `<expr>(<args>)`.
#[derive(Debug, TryClone, Parse, PartialEq, Eq, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ExprCall {
    /// Attributes associated with expression.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The name of the function being called.
    #[rune(meta)]
    pub expr: Box<ast::Expr>,
    /// The arguments of the function call.
    pub args: ast::Parenthesized<ast::Expr, T![,]>,
    /// Opaque identifier related with call.
    #[rune(skip)]
    pub(crate) id: ItemId,
}

expr_parse!(Call, ExprCall, "call expression");
