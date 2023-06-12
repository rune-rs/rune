use crate::ast;

/// Test if the given expression qualifieis as a block end or not, as with a
/// body in a match expression.
///
/// This determines if a comma is necessary or not after the expression.
pub(crate) fn is_block_end(expr: &ast::Expr, comma: Option<&T![,]>) -> bool {
    match (expr, comma) {
        (ast::Expr::Block(..), _) => false,
        (ast::Expr::For(..), _) => false,
        (ast::Expr::While(..), _) => false,
        (ast::Expr::If(..), _) => false,
        (ast::Expr::Match(..), _) => false,
        (_, Some(..)) => false,
        (_, None) => true,
    }
}
