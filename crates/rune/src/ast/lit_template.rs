use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A string template.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitTemplate>("`hello world`");
/// testing::roundtrip::<ast::LitTemplate>("`hello\\n world`");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
pub struct LitTemplate {
    /// The `template` keyword.
    pub template: T![template],
    /// Arguments to the template.
    pub args: ast::Braced<ast::Expr, T![,]>,
}
