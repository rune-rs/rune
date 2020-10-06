use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Spanned, ToTokens};

/// A `break` statement: `break [expr]`.
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprBreak>("break");
/// testing::roundtrip::<ast::ExprBreak>("break 42");
/// testing::roundtrip::<ast::ExprBreak>("#[attr] break 42");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
pub struct ExprBreak {
    /// The attributes of the `break` expression
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub break_token: ast::Break,
    /// An optional expression to break with.
    #[rune(iter)]
    pub expr: Option<ExprBreakValue>,
}

expr_parse!(ExprBreak, "break expression");

/// Things that we can break on.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
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
