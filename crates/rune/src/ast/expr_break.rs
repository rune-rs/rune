use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprBreak>("break");
    rt::<ast::ExprBreak>("break 42");
    rt::<ast::ExprBreak>("#[attr] break 42");
}

/// A break expression.
///
/// * `break [expr]`.
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ExprBreak {
    /// The attributes of the `break` expression
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub break_token: T![break],
    /// An optional expression to break with.
    #[rune(iter)]
    pub expr: Option<Box<ExprBreakValue>>,
}

expr_parse!(Break, ExprBreak, "break expression");

/// Things that we can break on.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum ExprBreakValue {
    /// Breaking a value out of a loop.
    Expr(ast::Expr),
    /// Break and jump to the given label.
    Label(ast::Label),
}

impl Parse for ExprBreakValue {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
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
