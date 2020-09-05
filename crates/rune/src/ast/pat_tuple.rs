use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::Span;

/// A tuple pattern.
#[derive(Debug, Clone)]
pub struct PatTuple {
    /// The path, if the tuple is typed.
    pub path: Option<ast::Path>,
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
        if let Some(path) = &self.path {
            path.span().join(self.close.span())
        } else {
            self.open.span().join(self.close.span())
        }
    }

    /// Parse a tuple pattern with a known preceeding path.
    pub fn parse_with_path(
        parser: &mut Parser<'_>,
        path: Option<ast::Path>,
    ) -> Result<Self, ParseError> {
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
            path,
            open,
            items,
            open_pattern,
            close,
        })
    }
}

impl Parse for PatTuple {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let path = if parser.peek::<ast::Path>()? {
            Some(parser.parse()?)
        } else {
            None
        };

        Self::parse_with_path(parser, path)
    }
}
