use crate::ast::{Colon, Expr, ExprBlock, Label, While};
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use st::unit::Span;

/// A let expression `let <name> = <expr>;`
#[derive(Debug, Clone)]
pub struct ExprWhile {
    /// A label for the while loop.
    pub label: Option<(Label, Colon)>,
    /// The `while` keyword.
    pub while_: While,
    /// The name of the binding.
    pub condition: Box<Expr>,
    /// The body of the while loop.
    pub body: Box<ExprBlock>,
}

impl ExprWhile {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.while_.token.span.join(self.body.span())
    }

    /// Parse with the given label.
    pub fn parse_with_label(
        parser: &mut Parser<'_>,
        label: Option<(Label, Colon)>,
    ) -> Result<Self, ParseError> {
        Ok(ExprWhile {
            label,
            while_: parser.parse()?,
            condition: Box::new(parser.parse()?),
            body: Box::new(parser.parse()?),
        })
    }
}

impl Parse for ExprWhile {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let label = if parser.peek::<Label>()? {
            Some((parser.parse()?, parser.parse()?))
        } else {
            None
        };

        Self::parse_with_label(parser, label)
    }
}
