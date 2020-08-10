use crate::ast;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// An object pattern.
#[derive(Debug, Clone)]
pub struct PatObject {
    /// The open object marker.
    pub open: ast::StartObject,
    /// The items matched against.
    pub items: Vec<(ast::LitObjectKey, ast::Colon, ast::Pat, Option<ast::Comma>)>,
    /// Indicates if the pattern is open or not.
    pub open_pattern: Option<ast::DotDot>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl PatObject {
    /// Get the span of the pattern.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

impl Parse for PatObject {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let open = parser.parse()?;
        let mut items = Vec::new();

        let mut is_open = true;

        while !parser.peek::<ast::CloseBrace>()? && !parser.peek::<ast::DotDot>()? {
            let key = parser.parse()?;
            let colon = parser.parse()?;
            let pat = parser.parse()?;

            is_open = parser.peek::<ast::Comma>()?;

            if !is_open {
                items.push((key, colon, pat, None));
                break;
            }

            items.push((key, colon, pat, Some(parser.parse()?)));
        }

        let open_pattern = if is_open && parser.peek::<ast::DotDot>()? {
            Some(parser.parse()?)
        } else {
            None
        };

        let close = parser.parse()?;

        Ok(Self {
            open,
            items,
            close,
            open_pattern,
        })
    }
}
