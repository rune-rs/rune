use crate::ast;
use crate::{IntoTokens, MacroContext, Parse, ParseError, Parser, Spanned, TokenStream};
use runestick::Span;

/// A struct declaration.
#[derive(Debug, Clone)]
pub struct ItemStruct {
    /// The attributes for the struct
    pub attributes: Vec<ast::Attribute>,
    /// The `struct` keyword.
    pub struct_: ast::Struct,
    /// The identifier of the struct declaration.
    pub ident: ast::Ident,
    /// The body of the struct.
    pub body: ItemStructBody,
}

impl ItemStruct {
    /// Parse a `struct` item with the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            struct_: parser.parse()?,
            ident: parser.parse()?,
            body: parser.parse()?,
        })
    }
}

impl Spanned for ItemStruct {
    fn span(&self) -> Span {
        let start = self.struct_.span();

        match &self.body {
            ItemStructBody::EmptyBody(semi) => start.join(semi.span()),
            ItemStructBody::TupleBody(_, semi) => start.join(semi.span()),
            ItemStructBody::StructBody(body) => start.join(body.span()),
        }
    }
}

/// Parse implementation for a struct.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ItemStruct>("struct Foo;").unwrap();
/// parse_all::<ast::ItemStruct>("struct Foo ( a, b, c );").unwrap();
/// parse_all::<ast::ItemStruct>("struct Foo { a, b, c }").unwrap();
/// parse_all::<ast::ItemStruct>("struct Foo { #[default_value = 1] a, b, c }").unwrap();
/// parse_all::<ast::ItemStruct>("#[alpha] struct Foo ( #[default_value = \"x\" ] a, b, c );").unwrap();
/// ```
impl Parse for ItemStruct {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}

impl IntoTokens for ItemStruct {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.attributes.into_tokens(context, stream);
        self.struct_.into_tokens(context, stream);
        self.ident.into_tokens(context, stream);
        self.body.into_tokens(context, stream);
    }
}

/// A struct declaration.
#[derive(Debug, Clone)]
pub enum ItemStructBody {
    /// An empty struct declaration.
    EmptyBody(ast::SemiColon),
    /// A tuple struct body.
    TupleBody(TupleBody, ast::SemiColon),
    /// A regular struct body.
    StructBody(StructBody),
}

/// Parse implementation for a struct body.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ItemStructBody>(";").unwrap();
/// parse_all::<ast::ItemStructBody>("( a, b, c );").unwrap();
/// parse_all::<ast::ItemStructBody>("();").unwrap();
/// parse_all::<ast::ItemStructBody>("{ a, b, c }").unwrap();
/// parse_all::<ast::ItemStructBody>("{ #[x] a, #[y] b, #[z] #[w] #[f32] c }").unwrap();
/// ```
impl Parse for ItemStructBody {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let t = parser.token_peek_eof()?;

        let body = match t.kind {
            ast::Kind::Open(ast::Delimiter::Parenthesis) => {
                Self::TupleBody(parser.parse()?, parser.parse()?)
            }
            ast::Kind::Open(ast::Delimiter::Brace) => Self::StructBody(parser.parse()?),
            _ => Self::EmptyBody(parser.parse()?),
        };

        Ok(body)
    }
}

impl IntoTokens for ItemStructBody {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        match self {
            ItemStructBody::EmptyBody(semi) => {
                semi.into_tokens(context, stream);
            }
            ItemStructBody::TupleBody(body, semi) => {
                body.into_tokens(context, stream);
                semi.into_tokens(context, stream);
            }
            ItemStructBody::StructBody(body) => {
                body.into_tokens(context, stream);
            }
        }
    }
}

/// A variant declaration.
#[derive(Debug, Clone)]
pub struct TupleBody {
    /// The opening paren.
    pub open: ast::OpenParen,
    /// Fields in the variant.
    pub fields: Vec<(Vec<ast::Attribute>, ast::Ident, Option<ast::Comma>)>,
    /// The close paren.
    pub close: ast::CloseParen,
}

impl TupleBody {
    /// Get the span for the tuple body.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

/// Parse implementation for a struct body.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::TupleBody>("( a, b, c )").unwrap();
/// parse_all::<ast::TupleBody>("( #[x] a, b, c )").unwrap();
/// ```
impl Parse for TupleBody {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut fields = Vec::new();

        while !parser.peek::<ast::CloseParen>()? {
            let attrs = parser.parse()?;
            let field = parser.parse()?;

            let comma = if parser.peek::<ast::Comma>()? {
                Some(parser.parse()?)
            } else {
                None
            };

            let done = comma.is_none();

            fields.push((attrs, field, comma));

            if done {
                break;
            }
        }

        Ok(Self {
            open,
            fields,
            close: parser.parse()?,
        })
    }
}

impl IntoTokens for TupleBody {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.open.into_tokens(context, stream);

        for (attrs, field, comma) in &self.fields {
            attrs.into_tokens(context, stream);
            field.into_tokens(context, stream);
            comma.into_tokens(context, stream);
        }

        self.close.into_tokens(context, stream);
    }
}

/// A variant declaration.
#[derive(Debug, Clone)]
pub struct StructBody {
    /// The opening brace.
    pub open: ast::OpenBrace,
    /// Fields in the variant.
    pub fields: Vec<(Vec<ast::Attribute>, ast::Ident, Option<ast::Comma>)>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl StructBody {
    /// Get the span for the tuple body.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

/// Parse implementation for a struct body.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::StructBody>("{ a, #[attribute] b, c }").unwrap();
/// ```
impl Parse for StructBody {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut fields = Vec::new();

        while !parser.peek::<ast::CloseBrace>()? {
            let attrs = parser.parse()?;
            let field = parser.parse()?;

            let comma = if parser.peek::<ast::Comma>()? {
                Some(parser.parse()?)
            } else {
                None
            };

            let done = comma.is_none();

            fields.push((attrs, field, comma));

            if done {
                break;
            }
        }

        let close = parser.parse()?;

        Ok(Self {
            open,
            fields,
            close,
        })
    }
}

impl IntoTokens for StructBody {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.open.into_tokens(context, stream);

        for (attrs, field, comma) in &self.fields {
            attrs.into_tokens(context, stream);
            field.into_tokens(context, stream);
            comma.into_tokens(context, stream);
        }

        self.close.into_tokens(context, stream);
    }
}
