use crate::ast::prelude::*;

/// A `yield [expr]` expression to return a value from a generator.
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
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
pub struct ExprYield {
    /// The attributes of the `yield`
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub yield_token: T![yield],
    /// An optional expression to yield.
    #[rune(iter)]
    pub expr: Option<ast::Expr>,
}

expr_parse!(Yield, ExprYield, "yield expression");
