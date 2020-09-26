use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A `yield` statement to return a value from a generator: `yield [expr]`.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprYield>("yield");
/// testing::roundtrip::<ast::ExprYield>("yield 42");
/// testing::roundtrip::<ast::ExprYield>("#[attr] yield 42");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprYield {
    /// The attributes of the `yield`
    #[rune(iter, attributes)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub yield_: ast::Yield,
    /// An optional expression to yield.
    #[rune(iter)]
    pub expr: Option<Box<ast::Expr>>,
}
