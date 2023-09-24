use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ItemEnum>("enum Foo { Bar(a), Baz(b), Empty() }");
    rt::<ast::ItemEnum>("enum Foo { Bar(a), Baz(b), #[default_value = \"zombie\"] Empty() }");
    rt::<ast::ItemEnum>(
        "#[repr(Rune)] enum Foo { Bar(a), Baz(b), #[default_value = \"zombie\"] Empty() }",
    );
    rt::<ast::ItemEnum>("pub enum Color { Blue, Red, Green }");

    rt::<ast::Fields>("( a, b, c )");
    rt::<ast::Fields>("{ a, b, c }");
    rt::<ast::Fields>("( #[serde(default)] a, b, c )");
    rt::<ast::Fields>("{ a, #[debug(skip)] b, c }");
}

/// An enum item.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ItemEnum {
    /// The attributes for the enum block
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `enum` item
    #[rune(option, meta)]
    pub visibility: ast::Visibility,
    /// The `enum` token.
    pub enum_token: T![enum],
    /// The name of the enum.
    pub name: ast::Ident,
    /// Variants in the enum.
    pub variants: ast::Braced<ItemVariant, T![,]>,
}

item_parse!(Enum, ItemEnum, "enum item");

/// An enum variant.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned, Opaque)]
#[non_exhaustive]
pub struct ItemVariant {
    /// Opaque identifier of variant.
    #[rune(id)]
    pub(crate) id: Id,
    /// The attributes associated with the variant.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The name of the variant.
    pub name: ast::Ident,
    /// The body of the variant.
    #[rune(iter)]
    pub body: ast::Fields,
}
