use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprAssign>("a = 2");
    rt::<ast::ExprAssign>("a = b = 3");
}

/// An assign expression.
///
/// * `a = b`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprAssign {
    /// Attributes associated with the assign expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expression being assigned to.
    pub lhs: Box<ast::Expr>,
    /// The equals sign `=`.
    pub eq: T![=],
    /// The value.
    pub rhs: Box<ast::Expr>,
}

expr_parse!(Assign, ExprAssign, "assign expression");
