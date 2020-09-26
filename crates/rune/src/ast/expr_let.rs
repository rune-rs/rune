use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A let expression `let <name> = <expr>;`
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprLet>("let x = 1");
/// testing::roundtrip::<ast::ExprLet>("#[attr] let a = f()");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprLet {
    /// The attributes for the let expression
    #[rune(iter, attributes)]
    pub attributes: Vec<ast::Attribute>,
    /// The `let` keyword.
    pub let_: ast::Let,
    /// The name of the binding.
    pub pat: ast::Pat,
    /// The equality keyword.
    pub eq: ast::Eq,
    /// The expression the binding is assigned to.
    pub expr: Box<ast::Expr>,
}

impl ExprLet {
    /// Parse a let expression without eager bracing.
    pub fn parse_without_eager_brace(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            attributes: vec![],
            let_: parser.parse()?,
            pat: parser.parse()?,
            eq: parser.parse()?,
            expr: Box::new(ast::Expr::parse_without_eager_brace(parser)?),
        })
    }
}
