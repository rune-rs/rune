use crate::ast::prelude::*;

/// A local variable declaration `let <pattern> = <expr>;`
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Local>("let x = 1;");
/// testing::roundtrip::<ast::Local>("#[attr] let a = f();");
/// testing::roundtrip::<ast::Local>("let a = b{}().foo[0].await;");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
#[non_exhaustive]
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
    #[rune(parse_with = "parse_expr")]
    pub expr: ast::Expr,
    /// Trailing semicolon of the local.
    pub semi: T![;],
}

fn parse_expr(p: &mut Parser<'_>) -> Result<ast::Expr, ParseError> {
    ast::Expr::parse_with(
        p,
        ast::expr::EAGER_BRACE,
        ast::expr::EAGER_BINARY,
        ast::expr::CALLABLE,
    )
}
