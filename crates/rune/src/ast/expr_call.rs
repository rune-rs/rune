use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    rt::<ast::ExprCall>("test()");
    rt::<ast::ExprCall>("(foo::bar)()");
}

/// A call expression.
///
/// * `<expr>(<args>)`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprCall {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The name of the function being called.
    pub expr: Box<ast::Expr>,
    /// The arguments of the function call.
    pub args: ast::Parenthesized<ast::Expr, T![,]>,
    /// Opaque identifier related with call.
    #[rune(skip)]
    pub(crate) id: ItemId,
}

expr_parse!(Call, ExprCall, "call expression");
