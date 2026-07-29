use crate::ast::prelude::*;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
    rt::<ast::ItemStatic>("static value = #{}");
    rt::<ast::ItemStatic>("static value");
}

/// A static declaration.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ItemStatic {
    /// The *inner* attributes that are applied to the static declaration.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the static.
    #[rune(option, meta)]
    pub visibility: ast::Visibility,
    /// The `static` keyword.
    #[rune(meta)]
    pub static_token: T![static],
    /// The name of the static.
    pub name: ast::Ident,
    /// The initializer, if the static has one.
    ///
    /// A static without an initializer has to be assigned before it can be
    /// read, either by a script or by the caller through the storage the
    /// virtual machine has been configured with.
    #[rune(iter)]
    pub init: Option<ItemStaticInit>,
    /// Opaque identifier for the static.
    #[rune(skip)]
    pub(crate) id: ItemId,
}

impl ItemStatic {}

/// The initializer of a [`ItemStatic`].
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ItemStaticInit {
    /// The equals token.
    pub eq: T![=],
    /// The expression the static is initialized with, which has to be
    /// constant.
    pub expr: ast::Expr,
}

impl Peek for ItemStaticInit {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K![=])
    }
}

item_parse!(Static, ItemStatic, "static item");
