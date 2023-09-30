use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprReturn>("return");
    rt::<ast::ExprReturn>("return 42");
    rt::<ast::ExprReturn>("#[attr] return 42");
}

/// A return expression.
///
/// * `return [expr]`.
#[derive(Debug, TryClone, Parse, PartialEq, Eq, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ExprReturn {
    /// The attributes of the `return` statement.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub return_token: T![return],
    /// An optional expression to return.
    #[rune(iter)]
    pub expr: Option<Box<ast::Expr>>,
}

expr_parse!(Return, ExprReturn, "return expression");
