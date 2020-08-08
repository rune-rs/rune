use crate::ast;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// A tuple pattern.
#[derive(Debug, Clone)]
pub struct PatTuple {
    /// The open bracket.
    pub open: ast::OpenParen,
    /// The numbers matched against.
    pub items: Vec<(Box<ast::Pat>, Option<ast::Comma>)>,
    /// Indicates if the pattern is open or not.
    pub open_pattern: Option<ast::DotDot>,
    /// The close bracket.
    pub close: ast::CloseParen,
}

impl PatTuple {
    /// Get the span of the pattern.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

impl Parse for PatTuple {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let open = parser.parse()?;
        let mut items = Vec::new();

        let mut is_open = true;

        while !parser.peek::<ast::CloseParen>()? && !parser.peek::<ast::DotDot>()? {
            let pat = parser.parse()?;

            is_open = parser.peek::<ast::Comma>()?;

            if !is_open {
                items.push((Box::new(pat), None));
                break;
            }

            items.push((Box::new(pat), Some(parser.parse()?)));
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
            open_pattern,
            close,
        })
    }
}
