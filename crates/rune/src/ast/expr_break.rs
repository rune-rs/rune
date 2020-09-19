use crate::ast;
use crate::{Ast, Parse, ParseError, Parser, Peek, Spanned};

/// A return statement `break [expr]`.
#[derive(Debug, Clone, Ast, Parse, Spanned)]
pub struct ExprBreak {
    /// The return token.
    pub break_: ast::Break,
    /// An optional expression to break with.
    #[spanned(last)]
    pub expr: Option<ExprBreakValue>,
}

/// Things that we can break on.
#[derive(Debug, Clone, Ast, Spanned)]
pub enum ExprBreakValue {
    /// Breaking a value out of a loop.
    Expr(Box<ast::Expr>),
    /// Break and jump to the given label.
    Label(ast::Label),
}

impl Parse for ExprBreakValue {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            ast::Kind::Label(..) => Self::Label(parser.parse()?),
            _ => Self::Expr(Box::new(parser.parse()?)),
        })
    }
}

impl Peek for ExprBreakValue {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        match t1.map(|t| t.kind) {
            Some(ast::Kind::Label(..)) => true,
            _ => ast::Expr::peek(t1, t2),
        }
    }
}
