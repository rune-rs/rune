use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprBreak>("break");
    rt::<ast::ExprBreak>("break 42");
    rt::<ast::ExprBreak>("#[attr] break 42");
}

/// A break expression.
///
/// * `break [expr]`.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ExprBreak {
    /// The attributes of the `break` expression
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub break_token: T![break],
    /// A label to break to.
    #[rune(iter)]
    pub label: Option<ast::Label>,
    /// An expression to break with.
    #[rune(iter)]
    pub expr: Option<Box<ast::Expr>>,
}

expr_parse!(Break, ExprBreak, "break expression");
