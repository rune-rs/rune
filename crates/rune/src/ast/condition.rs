use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::Condition>("true");
    rt::<ast::Condition>("let [a, ..] = v");
}

/// The condition in an if statement.
///
/// * `true`.
/// * `let Some(<pat>) = <expr>`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum Condition {
    /// A regular expression.
    Expr(ast::Expr),
    /// A pattern match.
    ExprLet(ast::ExprLet),
}

impl Parse for Condition {
    fn parse(p: &mut Parser) -> Result<Self> {
        Ok(match p.nth(0)? {
            K![let] => Self::ExprLet(ast::ExprLet::parse_without_eager_brace(p)?),
            _ => Self::Expr(ast::Expr::parse_without_eager_brace(p)?),
        })
    }
}
