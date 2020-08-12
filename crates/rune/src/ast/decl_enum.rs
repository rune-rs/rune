use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// An enum declaration.
#[derive(Debug, Clone)]
pub struct DeclEnum {
    /// The `enum` token.
    pub enum_: ast::Enum,
    /// The name of the enum.
    pub name: ast::Ident,
    /// The open brace of the declaration.
    pub open: ast::OpenBrace,
    /// Variants in the declaration.
    pub variants: Vec<(ast::Ident, ast::DeclStructBody, Option<ast::Comma>)>,
    /// The close brace in the declaration.
    pub close: ast::CloseBrace,
}

impl DeclEnum {
    /// Access the span for the enum declaration.
    pub fn span(&self) -> Span {
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
/// parse_all::<ast::DeclEnum>("enum Foo { Bar(a), Baz(b), Empty() }").unwrap();
/// ```
impl Parse for DeclEnum {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let enum_ = parser.parse()?;
        let name = parser.parse()?;
        let open = parser.parse()?;

        let mut variants = Vec::new();

        while !parser.peek::<ast::CloseBrace>()? {
            let name = parser.parse()?;
            let variant = parser.parse()?;

            let comma = if parser.peek::<ast::Comma>()? {
                Some(parser.parse()?)
            } else {
                None
            };

            let done = comma.is_none();

            variants.push((name, variant, comma));

            if done {
                break;
            }
        }

        let close = parser.parse()?;

        Ok(Self {
            enum_,
            name,
            open,
            variants,
            close,
        })
    }
}
