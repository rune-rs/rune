use crate::ast::{Colon, ExprBlock, Label, Loop};
use crate::{Parse, ParseError, Parser, Spanned};
use runestick::Span;

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

into_tokens!(ExprLoop { label, loop_, body });

impl ExprLoop {
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

impl Spanned for ExprLoop {
    fn span(&self) -> Span {
        self.loop_.token.span().join(self.body.span())
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
