use crate::ast::prelude::*;

/// A `continue` statement: `continue [label]`.
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::ExprContinue>("continue");
/// testing::roundtrip::<ast::ExprContinue>("continue 'foo");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
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
