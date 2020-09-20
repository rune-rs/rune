use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A let expression `let <name> = <expr>;`
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprLoop {
    /// A label followed by a colon.
    #[rune(iter)]
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

impl Parse for ExprLoop {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let label = parser.parse()?;
        Self::parse_with_label(parser, label)
    }
}
