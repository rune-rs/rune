use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::Expr>("(42).await");
    rt::<ast::Expr>("self.await");
    rt::<ast::Expr>("test.await");
}

/// An await expression.
///
/// * `<expr>.await`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprAwait {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expression being awaited.
    pub expr: Box<ast::Expr>,
    /// The dot separating the expression.
    pub dot: T![.],
    /// The await token.
    pub await_token: T![await],
}

expr_parse!(Await, ExprAwait, ".await expression");
