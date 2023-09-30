use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprContinue>("continue");
    rt::<ast::ExprContinue>("continue 'foo");
}

/// A `continue` statement.
///
/// * `continue [label]`.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ExprContinue {
    /// The attributes of the `break` expression
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub continue_token: T![continue],
    /// An optional label to continue to.
    #[rune(iter)]
    pub label: Option<ast::Label>,
}

expr_parse!(Continue, ExprContinue, "continue expression");
