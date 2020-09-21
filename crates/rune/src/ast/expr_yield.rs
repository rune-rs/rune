use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A `yield` statement to return a value from a generator: `yield [expr]`.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprYield>("yield").unwrap();
/// parse_all::<ast::ExprYield>("yield 42").unwrap();
/// parse_all::<ast::ExprYield>("#[attr] yield 42").unwrap();
/// ```
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
pub struct ExprYield {
    /// The attributes of the `yield`
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub yield_: ast::Yield,
    /// An optional expression to yield.
    #[rune(iter)]
    pub expr: Option<Box<ast::Expr>>,
}
