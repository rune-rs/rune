use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A return statement `return [expr]`.
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprReturn>("return");
/// testing::roundtrip::<ast::ExprReturn>("return 42");
/// testing::roundtrip::<ast::ExprReturn>("#[attr] return 42");
/// ```
#[derive(Debug, Clone, Parse, PartialEq, Eq, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
pub struct ExprReturn {
    /// The attributes of the `return` statement.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub return_token: T![return],
    /// An optional expression to return.
    #[rune(iter)]
    pub expr: Option<Box<ast::Expr>>,
}

expr_parse!(ExprReturn, "return expression");
