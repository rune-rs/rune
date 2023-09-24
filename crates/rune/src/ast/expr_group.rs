use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprGroup>("(for i in x {})");
    rt::<ast::ExprGroup>("(1 + 2)");
}

/// A prioritized expression group.
///
/// * `(<expr>)`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprGroup {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The open parenthesis.
    pub open: ast::OpenParen,
    /// The grouped expression.
    pub expr: Box<ast::Expr>,
    /// The close parenthesis.
    pub close: ast::CloseParen,
}

expr_parse!(Group, ExprGroup, "group expression");
