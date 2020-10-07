use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A local variable declaration `let <pattern> = <expr>;`
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Local>("let x = 1;");
/// testing::roundtrip::<ast::Local>("#[attr] let a = f();");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct Local {
    /// The attributes for the let expression
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The `let` keyword.
    pub let_token: T![let],
    /// The name of the binding.
    pub pat: ast::Pat,
    /// The equality keyword.
    pub eq: T![=],
    /// The expression the binding is assigned to.
    pub expr: Box<ast::Expr>,
    /// Trailing semicolon of the local.
    pub semi: T![;],
}
