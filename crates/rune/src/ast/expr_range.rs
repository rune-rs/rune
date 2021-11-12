use crate::ast::prelude::*;

/// A range expression `a .. b` or `a ..= b`.
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprRange>("0..42");
/// testing::roundtrip::<ast::ExprRange>("0..=42");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprRange {
    /// Attributes associated with the assign expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// Start of range.
    #[rune(iter)]
    pub from: Option<ast::Expr>,
    /// `..`.
    pub limits: ExprRangeLimits,
    /// End of range.
    #[rune(iter)]
    pub to: Option<ast::Expr>,
}

/// The limits of the specified range.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum ExprRangeLimits {
    /// Half-open range expression.
    HalfOpen(T![..]),
    /// Closed expression.
    Closed(T![..=]),
}

impl Parse for ExprRangeLimits {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K![..] => Self::HalfOpen(p.parse()?),
            K![..=] => Self::Closed(p.parse()?),
            _ => return Err(ParseError::expected(&p.tok_at(0)?, "range limits")),
        })
    }
}

expr_parse!(Range, ExprRange, "range expression");
