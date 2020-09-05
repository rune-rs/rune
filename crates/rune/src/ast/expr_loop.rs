use crate::ast::{Colon, ExprBlock, Label, Loop};
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// A let expression `let <name> = <expr>;`
#[derive(Debug, Clone)]
pub struct ExprLoop {
    /// A label followed by a colon.
    pub label: Option<(Label, Colon)>,
    /// The `loop` keyword.
    pub loop_: Loop,
    /// The body of the loop.
    pub body: Box<ExprBlock>,
}

impl ExprLoop {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.loop_.token.span.join(self.body.span())
    }

    /// Parse with the given label.
    pub fn parse_with_label(
        parser: &mut Parser<'_>,
        label: Option<(Label, Colon)>,
    ) -> Result<Self, ParseError> {
        Ok(ExprLoop {
            label,
            loop_: parser.parse()?,
            body: Box::new(parser.parse()?),
        })
    }
}

impl Parse for ExprLoop {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let label = if parser.peek::<Label>()? {
            Some((parser.parse()?, parser.parse()?))
        } else {
            None
        };

        Self::parse_with_label(parser, label)
    }
}
