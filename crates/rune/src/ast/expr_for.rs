use crate::ast::{Colon, Expr, ExprBlock, For, Ident, In, Label};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use stk::unit::Span;

/// A let expression `let <name> = <expr>;`
#[derive(Debug, Clone)]
pub struct ExprFor {
    /// The label of the loop.
    pub label: Option<(Label, Colon)>,
    /// The `for` keyword.
    pub for_: For,
    /// The variable binding.
    /// TODO: should be a pattern when that is supported.
    pub var: Ident,
    /// The `in` keyword.
    pub in_: In,
    /// Expression producing the iterator.
    pub iter: Box<Expr>,
    /// The body of the loop.
    pub body: Box<ExprBlock>,
}

impl ExprFor {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.for_.token.span.join(self.body.span())
    }

    /// Parse with the given label.
    pub fn parse_with_label(
        parser: &mut Parser<'_>,
        label: Option<(Label, Colon)>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            label,
            for_: parser.parse()?,
            var: parser.parse()?,
            in_: parser.parse()?,
            iter: Box::new(parser.parse()?),
            body: Box::new(parser.parse()?),
        })
    }
}

impl Parse for ExprFor {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let label = if parser.peek::<Label>()? {
            Some((parser.parse()?, parser.parse()?))
        } else {
            None
        };

        Self::parse_with_label(parser, label)
    }
}
