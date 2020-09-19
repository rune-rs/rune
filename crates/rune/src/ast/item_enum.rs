use crate::ast;
use crate::{IntoTokens, MacroContext, Parse, ParseError, Parser, Spanned, TokenStream};
use runestick::Span;

/// An enum declaration.
#[derive(Debug, Clone)]
pub struct ItemEnum {
    /// The attributes for the enum block
    pub attributes: Vec<ast::Attribute>,
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

into_tokens!(ItemEnum {
    attributes,
    enum_,
    name,
    open,
    variants,
    close,
});

impl ItemEnum {
    /// Parse a `enum` item with the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
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
            enum_,
            name,
            open,
            variants,
            close,
        })
    }
}

impl Spanned for ItemEnum {
    fn span(&self) -> Span {
        if let Some(first) = self.attributes.first() {
            first.span().join(self.close.span())
        } else {
            self.enum_.span().join(self.close.span())
        }
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
/// ```
impl Parse for ItemEnum {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}

/// An enum variant.
#[derive(Debug, Clone)]
pub struct ItemVariant {
    /// The attributes associated with the variant.
    pub attributes: Vec<ast::Attribute>,
    /// The name of the variant.
    pub name: ast::Ident,
    /// The body of the variant.
    pub body: ItemVariantBody,
    /// Optional trailing comma in variant.
    pub comma: Option<ast::Comma>,
}

into_tokens!(ItemVariant {
    attributes,
    name,
    body,
    comma,
});

impl Spanned for ItemVariant {
    fn span(&self) -> Span {
        let first = self
            .attributes
            .first()
            .map(Spanned::span)
            .unwrap_or_else(|| self.name.span());

        let last = self
            .comma
            .as_ref()
            .map(Spanned::span)
            .unwrap_or_else(|| match &self.body {
                ItemVariantBody::EmptyBody => self.name.span(),
                ItemVariantBody::TupleBody(body) => body.span(),
                ItemVariantBody::StructBody(body) => body.span(),
            });

        first.join(last)
    }
}

/// An item body declaration.
#[derive(Debug, Clone)]
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

impl IntoTokens for ItemVariantBody {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        match self {
            Self::EmptyBody => (),
            Self::TupleBody(body) => {
                body.into_tokens(context, stream);
            }
            Self::StructBody(body) => {
                body.into_tokens(context, stream);
            }
        }
    }
}
