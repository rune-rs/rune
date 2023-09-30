use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprRange>("0..42");
    rt::<ast::ExprRange>("0..=42");
    rt::<ast::ExprRange>("0..=a + 2");
}

/// A range expression.
///
/// * `a .. b` or `a ..= b`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprRange {
    /// Attributes associated with the assign expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// Start of range.
    #[rune(iter)]
    pub start: Option<Box<ast::Expr>>,
    /// The range limits.
    pub limits: ExprRangeLimits,
    /// End of range.
    #[rune(iter)]
    pub end: Option<Box<ast::Expr>>,
}

/// The limits of the specified range.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum ExprRangeLimits {
    /// Half-open range expression.
    HalfOpen(T![..]),
    /// Closed expression.
    Closed(T![..=]),
}

impl Parse for ExprRangeLimits {
    fn parse(p: &mut Parser) -> Result<Self> {
        Ok(match p.nth(0)? {
            K![..] => Self::HalfOpen(p.parse()?),
            K![..=] => Self::Closed(p.parse()?),
            _ => return Err(compile::Error::expected(p.tok_at(0)?, "range limits")),
        })
    }
}

expr_parse!(Range, ExprRange, "range expression");
