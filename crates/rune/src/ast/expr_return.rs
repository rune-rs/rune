use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A return statement `return [expr]`.
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprReturn>("return").unwrap();
/// parse_all::<ast::ExprReturn>("return 42").unwrap();
/// parse_all::<ast::ExprReturn>("#[attr] return 42").unwrap();
/// ```
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
pub struct ExprReturn {
    /// The attributes of the `return` statement.
    #[rune(iter, attributes)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub return_: ast::Return,
    /// An optional expression to return.
    #[rune(iter)]
    pub expr: Option<Box<ast::Expr>>,
}
