use crate::ast::prelude::*;

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
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        let token = p.next()?;

        let value = match token.kind {
            K![true] => true,
            K![false] => false,
            _ => {
                return Err(ParseError::expected(
                    &token,
                    "boolean literal `true` or `false`",
                ));
            }
        };

        Ok(Self { value, token })
    }
}

impl Peek for LitBool {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K![true] | K![false])
    }
}
