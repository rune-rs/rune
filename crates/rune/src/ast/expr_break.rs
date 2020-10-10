use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Peeker, Spanned, ToTokens};

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
    pub break_token: T![break],
    /// An optional expression to break with.
    #[rune(iter)]
    pub expr: Option<ExprBreakValue>,
}

expr_parse!(Break, ExprBreak, "break expression");

/// Things that we can break on.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum ExprBreakValue {
    /// Breaking a value out of a loop.
    Expr(ast::Expr),
    /// Break and jump to the given label.
    Label(ast::Label),
}

impl Parse for ExprBreakValue {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K!['label] => Self::Label(p.parse()?),
            _ => Self::Expr(p.parse()?),
        })
    }
}

impl Peek for ExprBreakValue {
    fn peek(p: &mut Peeker<'_>) -> bool {
        match p.nth(0) {
            K!['label] => true,
            _ => ast::Expr::peek(p),
        }
    }
}
