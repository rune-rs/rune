use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Spanned, ToTokens};

/// A `break` statement: `break [expr]`.
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprBreak>("break").unwrap();
/// parse_all::<ast::ExprBreak>("break 42").unwrap();
/// parse_all::<ast::ExprBreak>("#[attr] break 42").unwrap();
/// ```
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
pub struct ExprBreak {
    /// The attributes of the `break` expression
    #[rune(iter, attributes)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub break_: ast::Break,
    /// An optional expression to break with.
    #[rune(iter)]
    pub expr: Option<ExprBreakValue>,
}

/// Things that we can break on.
#[derive(Debug, Clone, ToTokens, Spanned)]
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
