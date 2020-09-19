use crate::ast;
use crate::{Ast, Parse, Spanned};

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
#[derive(Debug, Clone, Ast, Parse, Spanned)]
pub struct ExprYield {
    /// The return token.
    pub yield_: ast::Yield,
    /// An optional expression to yield.
    #[spanned(iter)]
    pub expr: Option<Box<ast::Expr>>,
}
