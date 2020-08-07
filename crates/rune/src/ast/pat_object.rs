use crate::ast::{CloseBrace, Colon, Comma, DotDot, LitStr, Pat, StartObject};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// An array pattern.
#[derive(Debug, Clone)]
pub struct PatObject {
    /// The open object marker.
    pub open: StartObject,
    /// The items matched against.
    pub items: Vec<(LitStr, Colon, Box<Pat>, Option<Comma>)>,
    /// Indicates if the pattern is open or not.
    pub open_pattern: Option<DotDot>,
    /// The close brace.
    pub close: CloseBrace,
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

        while !parser.peek::<CloseBrace>()? && !parser.peek::<DotDot>()? {
            let key = parser.parse()?;
            let colon = parser.parse()?;
            let pat = parser.parse()?;

            is_open = parser.peek::<Comma>()?;

            if !is_open {
                items.push((key, colon, Box::new(pat), None));
                break;
            }

            items.push((key, colon, Box::new(pat), Some(parser.parse()?)));
        }

        let open_pattern = if is_open && parser.peek::<DotDot>()? {
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
