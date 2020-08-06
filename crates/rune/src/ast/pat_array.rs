use crate::ast::{CloseBracket, Comma, DotDot, OpenBracket, Pat};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use stk::unit::Span;

/// An array pattern.
#[derive(Debug, Clone)]
pub struct PatArray {
    /// The open bracket.
    pub open: OpenBracket,
    /// The numbers matched against.
    pub items: Vec<(Box<Pat>, Option<Comma>)>,
    /// Indicates if the pattern is open or not.
    pub open_pattern: Option<DotDot>,
    /// The close bracket.
    pub close: CloseBracket,
}

impl PatArray {
    /// Get the span of the pattern.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

impl Parse for PatArray {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let open = parser.parse()?;
        let mut items = Vec::new();

        let mut is_open = true;

        while !parser.peek::<CloseBracket>()? && !parser.peek::<DotDot>()? {
            let pat = parser.parse()?;

            is_open = parser.peek::<Comma>()?;

            if !is_open {
                items.push((Box::new(pat), None));
                break;
            }

            items.push((Box::new(pat), Some(parser.parse()?)));
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
            open_pattern,
            close,
        })
    }
}
