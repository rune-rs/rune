use crate::ast;
use crate::{ParseError, Parser, Spanned, ToTokens};

/// A `yield [expr]` expression to return a value from a generator.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprYield>("yield");
/// testing::roundtrip::<ast::ExprYield>("yield 42");
/// testing::roundtrip::<ast::ExprYield>("#[attr] yield 42");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprYield {
    /// The attributes of the `yield`
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The return token.
    pub yield_token: ast::Yield,
    /// An optional expression to yield.
    #[rune(iter)]
    pub expr: Option<Box<ast::Expr>>,
}

impl ExprYield {
    /// Parse the yield expression with the given attributes.
    pub(crate) fn parse_with_attributes(
        parser: &mut Parser,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            yield_token: parser.parse()?,
            expr: parser.parse()?,
        })
    }
}

expr_parse!(ExprYield, "yield expression");
