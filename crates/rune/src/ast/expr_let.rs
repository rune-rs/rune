use crate::ast;
use crate::{Ast, Parse, ParseError, Parser, Spanned};

/// A let expression `let <name> = <expr>;`
#[derive(Debug, Clone, Ast, Parse, Spanned)]
pub struct ExprLet {
    /// The `let` keyword.
    pub let_: ast::Let,
    /// The name of the binding.
    pub pat: ast::Pat,
    /// The equality keyword.
    pub eq: ast::Eq,
    /// The expression the binding is assigned to.
    pub expr: Box<ast::Expr>,
}

impl ExprLet {
    /// Parse a let expression without eager bracing.
    pub fn parse_without_eager_brace(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            let_: parser.parse()?,
            pat: parser.parse()?,
            eq: parser.parse()?,
            expr: Box::new(ast::Expr::parse_without_eager_brace(parser)?),
        })
    }
}
