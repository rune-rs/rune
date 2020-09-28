use crate::ast;
use crate::{ParseError, Parser, Spanned, ToTokens};

/// A return statement `return [expr]`.
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprReturn>("return");
/// testing::roundtrip::<ast::ExprReturn>("return 42");
/// testing::roundtrip::<ast::ExprReturn>("#[attr] return 42");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprReturn {
    /// The attributes of the `return` statement.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub return_token: ast::Return,
    /// An optional expression to return.
    #[rune(iter)]
    pub expr: Option<Box<ast::Expr>>,
}

impl ExprReturn {
    /// Parse with the given attributes.
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            return_token: parser.parse()?,
            expr: parser.parse()?,
        })
    }
}

expr_parse!(ExprReturn, "return expression");
