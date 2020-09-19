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
    pub variants: Vec<(
        Vec<ast::Attribute>,
        ast::Ident,
        ItemEnumVariant,
        Option<ast::Comma>,
    )>,
    /// The close brace in the declaration.
    pub close: ast::CloseBrace,
}

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
            let attrs = parser.parse()?;
            let name = parser.parse()?;
            let variant = parser.parse()?;

            let comma = if parser.peek::<ast::Comma>()? {
                Some(parser.parse()?)
            } else {
                None
            };

            let done = comma.is_none();

            variants.push((attrs, name, variant, comma));

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
        self.enum_.span().join(self.close.span())
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

impl IntoTokens for ItemEnum {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.enum_.into_tokens(context, stream);
        self.name.into_tokens(context, stream);
        self.open.into_tokens(context, stream);

        for (attrs, variant, body, comma) in &self.variants {
            attrs.into_tokens(context, stream);
            variant.into_tokens(context, stream);
            body.into_tokens(context, stream);
            comma.into_tokens(context, stream);
        }

        self.close.into_tokens(context, stream);
    }
}

/// An item body declaration.
#[derive(Debug, Clone)]
pub enum ItemEnumVariant {
    /// An empty enum body.
    EmptyBody,
    /// A tuple struct body.
    TupleBody(ast::TupleBody),
    /// A regular struct body.
    StructBody(ast::StructBody),
}

/// Parse implementation for a struct body.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ItemEnumVariant>("( a, b, c );").unwrap();
/// parse_all::<ast::ItemEnumVariant>("{ a, b, c }").unwrap();
/// parse_all::<ast::ItemEnumVariant>("( #[serde(default)] a, b, c );").unwrap();
/// parse_all::<ast::ItemEnumVariant>("{ a, #[debug(skip)] b, c }").unwrap();
/// ```
impl Parse for ItemEnumVariant {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek()?;

        Ok(match token.map(|t| t.kind) {
            Some(ast::Kind::Open(ast::Delimiter::Parenthesis)) => Self::TupleBody(parser.parse()?),
            Some(ast::Kind::Open(ast::Delimiter::Brace)) => Self::StructBody(parser.parse()?),
            _ => Self::EmptyBody,
        })
    }
}

impl IntoTokens for ItemEnumVariant {
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
