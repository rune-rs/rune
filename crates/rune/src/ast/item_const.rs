use crate::ast::prelude::*;

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
    pub const_token: T![const],
    /// The name of the constant.
    pub name: ast::Ident,
    /// The equals token.
    pub eq: T![=],
    /// The optional body of the module declaration.
    pub expr: ast::Expr,
}

impl ItemConst {
    /// Get the descriptive span of this item, e.g. `const ITEM` instead of the
    /// span for the whole expression.
    pub fn descriptive_span(&self) -> Span {
        self.const_token.span().join(self.name.span())
    }
}

item_parse!(Const, ItemConst, "constant item");
