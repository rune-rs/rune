use crate::ast;
use crate::{Id, Parse, Spanned, ToTokens};

/// A const declaration.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ItemConst>("const value = #{}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
pub struct ItemConst {
    /// Opaque identifier for the constant.
    #[rune(id)]
    pub id: Option<Id>,
    /// The *inner* attributes that are applied to the const declaration.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the const.
    #[rune(optional, meta)]
    pub visibility: ast::Visibility,
    /// The `const` keyword.
    #[rune(meta)]
    pub const_token: ast::Const,
    /// The name of the constant.
    pub name: ast::Ident,
    /// The equals token.
    pub eq: ast::Eq,
    /// The optional body of the module declaration.
    pub expr: Box<ast::Expr>,
}

item_parse!(ItemConst, "constant item");
