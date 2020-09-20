use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A return statement `break [expr]`.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprYield>("yield").unwrap();
/// parse_all::<ast::ExprYield>("yield 42").unwrap();
/// ```
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
pub struct ExprYield {
    /// The return token.
    pub yield_: ast::Yield,
    /// An optional expression to yield.
    #[rune(iter)]
    pub expr: Option<Box<ast::Expr>>,
}
