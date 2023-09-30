use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprYield>("yield");
    rt::<ast::ExprYield>("yield 42");
    rt::<ast::ExprYield>("#[attr] yield 42");
}

/// A `yield` expression to return a value from a generator.
///
/// * `yield [expr]`.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ExprYield {
    /// The attributes of the `yield`
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub yield_token: T![yield],
    /// An optional expression to yield.
    #[rune(iter)]
    pub expr: Option<Box<ast::Expr>>,
}

expr_parse!(Yield, ExprYield, "yield expression");
