use crate::ast::prelude::*;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
    rt::<ast::ItemConst>("const value = #{}");
}

/// A const declaration.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ItemConst {
    /// The *inner* attributes that are applied to the const declaration.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the const.
    #[rune(option, meta)]
    pub visibility: ast::Visibility,
    /// The `const` keyword.
    #[rune(meta)]
    pub const_token: T![const],
    /// The name of the constant.
    pub name: ast::Ident,
    /// The equals token.
    pub eq: T![=],
    /// The optional body of the module declaration.
    pub expr: ast::Expr,
    /// Opaque identifier for the constant.
    #[rune(skip)]
    pub(crate) id: ItemId,
}

impl ItemConst {}

item_parse!(Const, ItemConst, "constant item");
