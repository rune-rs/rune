use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A const declaration.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ItemConst>("const value = #{};").unwrap();
/// ```
#[derive(Debug, Clone, Parse, ToTokens, Spanned)]
pub struct ItemConst {
    /// The *inner* attributes that are applied to the const declaration.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The `const` keyword.
    pub const_token: ast::Const,
    /// The name of the constant.
    pub name: ast::Ident,
    /// The equals token.
    pub eq: ast::Eq,
    /// The optional body of the module declaration.
    pub expr: Box<ast::Expr>,
    /// Terminating semicolon.
    pub semi: ast::SemiColon,
}
