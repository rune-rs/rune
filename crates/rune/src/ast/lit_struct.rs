use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// A literal struct field.
#[derive(Debug, Clone)]
pub struct LitStructField {
    /// The key of the field.
    pub key: ast::Ident,
    /// Colon separator.
    pub colon: ast::Colon,
    /// The value of the field.
    pub value: ast::Expr,
}

impl Parse for LitStructField {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            key: parser.parse()?,
            colon: parser.parse()?,
            value: parser.parse()?,
        })
    }
}

/// An expression to construct a literal struct.
#[derive(Debug, Clone)]
pub struct LitStruct {
    /// The name of the struct being instantiated.
    pub path: ast::Path,
    /// The open bracket.
    pub open: ast::OpenBrace,
    /// Items in the struct.
    pub fields: Vec<(LitStructField, Option<ast::Comma>)>,
    /// The close bracket.
    pub close: ast::CloseBrace,
}

impl LitStruct {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.path.span().join(self.close.span())
    }

    /// Parse the literal struct with a leading path.
    pub fn parse_with_path(parser: &mut Parser<'_>, path: ast::Path) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut fields = Vec::new();

        while !parser.peek::<ast::CloseBrace>()? {
            let expr = parser.parse::<LitStructField>()?;

            let comma = if parser.peek::<ast::Comma>()? {
                Some(parser.parse::<ast::Comma>()?)
            } else {
                None
            };

            let is_end = comma.is_none();
            fields.push((expr, comma));

            if is_end {
                break;
            }
        }

        let close = parser.parse()?;

        Ok(Self {
            path,
            open,
            fields,
            close,
        })
    }
}

/// Parse a struct literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> rune::Result<()> {
/// parse_all::<ast::LitStruct>("Foo {}").unwrap();
/// parse_all::<ast::LitStruct>("Foo { a: 1, b: \"two\" }").unwrap();
/// # Ok(())
/// # }
/// ```
impl Parse for LitStruct {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let path = parser.parse()?;
        Self::parse_with_path(parser, path)
    }
}
