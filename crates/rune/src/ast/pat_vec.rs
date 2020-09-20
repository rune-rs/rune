use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// An array pattern.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct PatVec {
    /// The open bracket.
    pub open: ast::OpenBracket,
    /// The numbers matched against.
    pub items: Vec<(Box<ast::Pat>, Option<ast::Comma>)>,
    /// Indicates if the pattern is open or not.
    pub open_pattern: Option<ast::DotDot>,
    /// The close bracket.
    pub close: ast::CloseBracket,
}

impl Parse for PatVec {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let open = parser.parse()?;
        let mut items = Vec::new();

        let mut is_open = true;

        while !parser.peek::<ast::CloseBracket>()? && !parser.peek::<ast::DotDot>()? {
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
