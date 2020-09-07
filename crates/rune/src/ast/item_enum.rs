use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use crate::{IntoTokens, MacroContext, TokenStream};
use runestick::Span;

/// An enum declaration.
#[derive(Debug, Clone)]
pub struct ItemEnum {
    /// The `enum` token.
    pub enum_: ast::Enum,
    /// The name of the enum.
    pub name: ast::Ident,
    /// The open brace of the declaration.
    pub open: ast::OpenBrace,
    /// Variants in the declaration.
    pub variants: Vec<(ast::Ident, ast::ItemStructBody, Option<ast::Comma>)>,
    /// The close brace in the declaration.
    pub close: ast::CloseBrace,
}

impl ItemEnum {
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
/// parse_all::<ast::ItemEnum>("enum Foo { Bar(a), Baz(b), Empty() }").unwrap();
/// ```
impl Parse for ItemEnum {
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

impl IntoTokens for ItemEnum {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.enum_.into_tokens(context, stream);
        self.name.into_tokens(context, stream);
        self.open.into_tokens(context, stream);

        for (variant, body, comma) in &self.variants {
            variant.into_tokens(context, stream);
            body.into_tokens(context, stream);
            comma.into_tokens(context, stream);
        }

        self.close.into_tokens(context, stream);
    }
}
