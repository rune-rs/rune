use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// A for loop expression `for i in [1, 2, 3] {}`
#[derive(Debug, Clone)]
pub struct ExprFor {
    /// The label of the loop.
    pub label: Option<(ast::Label, ast::Colon)>,
    /// The `for` keyword.
    pub for_: ast::For,
    /// The variable binding.
    /// TODO: should be a pattern when that is supported.
    pub var: ast::Ident,
    /// The `in` keyword.
    pub in_: ast::In,
    /// Expression producing the iterator.
    pub iter: Box<ast::Expr>,
    /// The body of the loop.
    pub body: Box<ast::ExprBlock>,
}

into_tokens!(ExprFor {
    label,
    for_,
    var,
    in_,
    iter,
    body
});

impl ExprFor {
    /// Parse with the given label.
    pub fn parse_with_label(
        parser: &mut Parser<'_>,
        label: Option<(ast::Label, ast::Colon)>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            label,
            for_: parser.parse()?,
            var: parser.parse()?,
            in_: parser.parse()?,
            iter: Box::new(ast::Expr::parse_without_eager_brace(parser)?),
            body: Box::new(parser.parse()?),
        })
    }
}

impl Spanned for ExprFor {
    fn span(&self) -> Span {
        self.for_.token.span().join(self.body.span())
    }
}

impl Parse for ExprFor {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let label = if parser.peek::<ast::Label>()? {
            Some((parser.parse()?, parser.parse()?))
        } else {
            None
        };

        Self::parse_with_label(parser, label)
    }
}
