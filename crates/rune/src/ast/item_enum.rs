use crate::ast;
use crate::{OptionSpanned, Parse, ParseError, Parser, Spanned, ToTokens};

/// An enum declaration.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ItemEnum {
    /// The attributes for the enum block
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `enum` item
    #[rune(iter)]
    pub visibility: Option<ast::Visibility>,
    /// The `enum` token.
    pub enum_: ast::Enum,
    /// The name of the enum.
    pub name: ast::Ident,
    /// The open brace of the declaration.
    pub open: ast::OpenBrace,
    /// Variants in the declaration.
    pub variants: Vec<ItemVariant>,
    /// The close brace in the declaration.
    pub close: ast::CloseBrace,
}

impl ItemEnum {
    /// Parse a `enum` item with the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let visibility = parser.parse()?;
        let enum_ = parser.parse()?;
        let name = parser.parse()?;
        let open = parser.parse()?;

        let mut variants = Vec::new();

        while !parser.peek::<ast::CloseBrace>()? {
            let variant = ItemVariant {
                attributes: parser.parse()?,
                name: parser.parse()?,
                body: parser.parse()?,
                comma: parser.parse()?,
            };

            let done = variant.comma.is_none();
            variants.push(variant);

            if done {
                break;
            }
        }

        let close = parser.parse()?;

        Ok(Self {
            attributes,
            visibility,
            enum_,
            name,
            open,
            variants,
            close,
        })
    }
}

/// Parse implementation for an enum.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ItemEnum>("enum Foo { Bar(a), Baz(b), Empty() }").unwrap();
/// parse_all::<ast::ItemEnum>("enum Foo { Bar(a), Baz(b), #[default_value = \"zombie\"] Empty() }").unwrap();
/// parse_all::<ast::ItemEnum>("#[repr(Rune)] enum Foo { Bar(a), Baz(b), #[default_value = \"zombie\"] Empty() }").unwrap();
/// parse_all::<ast::ItemEnum>("pub enum Color { Blue, Red, Green }").unwrap();
/// ```
impl Parse for ItemEnum {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}

/// An enum variant.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ItemVariant {
    /// The attributes associated with the variant.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The name of the variant.
    pub name: ast::Ident,
    /// The body of the variant.
    #[rune(optional)]
    pub body: ItemVariantBody,
    /// Optional trailing comma in variant.
    #[rune(iter)]
    pub comma: Option<ast::Comma>,
}

/// An item body declaration.
#[derive(Debug, Clone, ToTokens, OptionSpanned)]
pub enum ItemVariantBody {
    /// An empty enum body.
    EmptyBody,
    /// A tuple struct body.
    TupleBody(ast::TupleBody),
    /// A regular struct body.
    StructBody(ast::StructBody),
}

impl ItemVariantBody {
    /// Iterate over the fields of the body.
    pub fn fields(&self) -> impl Iterator<Item = &'_ ast::Field> {
        match self {
            ItemVariantBody::EmptyBody => IntoIterator::into_iter(&[]),
            ItemVariantBody::TupleBody(body) => body.fields.iter(),
            ItemVariantBody::StructBody(body) => body.fields.iter(),
        }
    }
}

/// Parse implementation for a struct body.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ItemVariantBody>("( a, b, c );").unwrap();
/// parse_all::<ast::ItemVariantBody>("{ a, b, c }").unwrap();
/// parse_all::<ast::ItemVariantBody>("( #[serde(default)] a, b, c );").unwrap();
/// parse_all::<ast::ItemVariantBody>("{ a, #[debug(skip)] b, c }").unwrap();
/// ```
impl Parse for ItemVariantBody {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek()?;

        Ok(match token.map(|t| t.kind) {
            Some(ast::Kind::Open(ast::Delimiter::Parenthesis)) => Self::TupleBody(parser.parse()?),
            Some(ast::Kind::Open(ast::Delimiter::Brace)) => Self::StructBody(parser.parse()?),
            _ => Self::EmptyBody,
        })
    }
}
