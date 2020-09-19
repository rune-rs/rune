use crate::ast;
use crate::{Ast, Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// A struct declaration.
#[derive(Debug, Clone, Ast)]
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

/// A struct declaration.
#[derive(Debug, Clone, Ast)]
pub enum ItemStructBody {
    /// An empty struct declaration.
    EmptyBody(ast::SemiColon),
    /// A tuple struct body.
    TupleBody(TupleBody, ast::SemiColon),
    /// A regular struct body.
    StructBody(StructBody),
}

impl ItemStructBody {
    /// Iterate over the fields of the body.
    pub fn fields(&self) -> impl Iterator<Item = &'_ Field> {
        match self {
            ItemStructBody::EmptyBody(..) => IntoIterator::into_iter(&[]),
            ItemStructBody::TupleBody(body, ..) => body.fields.iter(),
            ItemStructBody::StructBody(body) => body.fields.iter(),
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

/// A variant declaration.
#[derive(Debug, Clone, Ast)]
pub struct TupleBody {
    /// The opening paren.
    pub open: ast::OpenParen,
    /// Fields in the variant.
    pub fields: Vec<Field>,
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
            let field = parser.parse::<Field>()?;
            let done = field.comma.is_none();
            fields.push(field);

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

/// A variant declaration.
#[derive(Debug, Clone, Ast)]
pub struct StructBody {
    /// The opening brace.
    pub open: ast::OpenBrace,
    /// Fields in the variant.
    pub fields: Vec<Field>,
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
            let field = parser.parse::<Field>()?;
            let done = field.comma.is_none();
            fields.push(field);

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

/// A field as part of a struct or a tuple body.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::Field>("a").unwrap();
/// parse_all::<ast::Field>("#[x] a").unwrap();
/// ```
#[derive(Debug, Clone, Ast, Parse)]
pub struct Field {
    /// Attributes associated with field.
    pub attributes: Vec<ast::Attribute>,
    /// Name of the field.
    pub name: ast::Ident,
    /// Trailing comma of the field.
    pub comma: Option<ast::Comma>,
}

impl Spanned for Field {
    fn span(&self) -> Span {
        let last = self
            .comma
            .as_ref()
            .map(Spanned::span)
            .unwrap_or_else(|| self.name.span());

        if let Some(first) = self.attributes.first() {
            first.span().join(last)
        } else {
            last
        }
    }
}
