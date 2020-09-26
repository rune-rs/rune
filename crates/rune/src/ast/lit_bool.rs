use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Spanned, ToTokens};

/// The unit literal `()`.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct LitBool {
    /// The token of the literal.
    pub token: ast::Token,
    /// The value of the literal.
    #[rune(skip)]
    pub value: bool,
}

/// Parsing a unit literal
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitBool>("true");
/// testing::roundtrip::<ast::LitBool>("false");
/// ```
impl Parse for LitBool {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        let value = match token.kind {
            ast::Kind::True => true,
            ast::Kind::False => false,
            _ => {
                return Err(ParseError::expected(
                    token,
                    "boolean literal `true` or `false`",
                ));
            }
        };

        Ok(Self { value, token })
    }
}

impl Peek for LitBool {
    fn peek(p1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        matches!(peek!(p1).kind, ast::Kind::True | ast::Kind::False)
    }
}
