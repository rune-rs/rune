use crate::ast;
use crate::{Id, OptionSpanned, Parse, ParseError, Parser, Spanned, ToTokens};

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
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ItemEnum {
    /// The attributes for the enum block
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `enum` item
    #[rune(optional)]
    pub visibility: ast::Visibility,
    /// The `enum` token.
    pub enum_token: ast::Enum,
    /// The name of the enum.
    pub name: ast::Ident,
    /// Variants in the enum.
    pub variants: ast::Braced<ItemVariant, ast::Comma>,
}

impl ItemEnum {
    /// Parse a `enum` item with the given attributes
    pub fn parse_with_meta(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        visibility: ast::Visibility,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            visibility,
            enum_token: parser.parse()?,
            name: parser.parse()?,
            variants: parser.parse()?,
        })
    }
}

item_parse!(ItemEnum, "enum item");

/// An enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
pub struct ItemVariant {
    /// Opaque identifier of variant.
    #[rune(id)]
    pub id: Id,
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
    TupleBody(ast::Parenthesized<ast::Field, ast::Comma>),
    /// A regular struct body.
    StructBody(ast::Braced<ast::Field, ast::Comma>),
}

impl ItemVariantBody {
    /// Iterate over the fields of the body.
    pub fn fields(&self) -> impl Iterator<Item = &'_ (ast::Field, Option<ast::Comma>)> {
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
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek()?;

        Ok(match token.map(|t| t.kind) {
            Some(ast::Kind::Open(ast::Delimiter::Parenthesis)) => Self::TupleBody(parser.parse()?),
            Some(ast::Kind::Open(ast::Delimiter::Brace)) => Self::StructBody(parser.parse()?),
            _ => Self::UnitBody,
        })
    }
}
