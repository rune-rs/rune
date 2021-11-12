use crate::ast::prelude::*;

/// An enum item.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ItemEnum>("enum Foo { Bar(a), Baz(b), Empty() }");
/// testing::roundtrip::<ast::ItemEnum>("enum Foo { Bar(a), Baz(b), #[default_value = \"zombie\"] Empty() }");
/// testing::roundtrip::<ast::ItemEnum>("#[repr(Rune)] enum Foo { Bar(a), Baz(b), #[default_value = \"zombie\"] Empty() }");
/// testing::roundtrip::<ast::ItemEnum>("pub enum Color { Blue, Red, Green }");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
pub struct ItemEnum {
    /// The attributes for the enum block
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `enum` item
    #[rune(optional, meta)]
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
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
pub struct ItemVariant {
    /// Opaque identifier of variant.
    #[rune(id)]
    pub id: Option<Id>,
    /// The attributes associated with the variant.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The name of the variant.
    pub name: ast::Ident,
    /// The body of the variant.
    #[rune(optional)]
    pub body: ItemVariantBody,
}

/// An item body declaration.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, OptionSpanned)]
pub enum ItemVariantBody {
    /// An empty enum body.
    UnitBody,
    /// A tuple struct body.
    TupleBody(ast::Parenthesized<ast::Field, T![,]>),
    /// A regular struct body.
    StructBody(ast::Braced<ast::Field, T![,]>),
}

impl ItemVariantBody {
    /// Iterate over the fields of the body.
    pub fn fields(&self) -> impl Iterator<Item = &'_ (ast::Field, Option<T![,]>)> {
        match self {
            ItemVariantBody::UnitBody => IntoIterator::into_iter(&[]),
            ItemVariantBody::TupleBody(body) => body.iter(),
            ItemVariantBody::StructBody(body) => body.iter(),
        }
    }
}

/// Parse implementation for a struct body.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ItemVariantBody>("( a, b, c )");
/// testing::roundtrip::<ast::ItemVariantBody>("{ a, b, c }");
/// testing::roundtrip::<ast::ItemVariantBody>("( #[serde(default)] a, b, c )");
/// testing::roundtrip::<ast::ItemVariantBody>("{ a, #[debug(skip)] b, c }");
/// ```
impl Parse for ItemVariantBody {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K!['('] => Self::TupleBody(p.parse()?),
            K!['{'] => Self::StructBody(p.parse()?),
            _ => Self::UnitBody,
        })
    }
}
