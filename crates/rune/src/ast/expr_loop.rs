use crate::ast;
use crate::{Ast, Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// A let expression `let <name> = <expr>;`
#[derive(Debug, Clone, Ast)]
pub struct ExprLoop {
    /// A label followed by a colon.
    pub label: Option<(ast::Label, ast::Colon)>,
    /// The `loop` keyword.
    pub loop_: ast::Loop,
    /// The body of the loop.
    pub body: Box<ast::ExprBlock>,
}

impl ExprLoop {
    /// Parse with the given label.
    pub fn parse_with_label(
        parser: &mut Parser<'_>,
        label: Option<(ast::Label, ast::Colon)>,
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
        let label = if parser.peek::<ast::Label>()? {
            Some((parser.parse()?, parser.parse()?))
        } else {
            None
        };

        Self::parse_with_label(parser, label)
    }
}
