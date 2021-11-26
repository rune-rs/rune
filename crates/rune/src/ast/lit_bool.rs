use crate::ast::prelude::*;

/// The unit literal `()`.
#[derive(Debug, Clone, PartialEq, Eq, Spanned)]
#[non_exhaustive]
pub struct LitBool {
    /// The span corresponding to the literal.
    pub span: Span,
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
        let t = p.next()?;

        let value = match t.kind {
            K![true] => true,
            K![false] => false,
            _ => {
                return Err(ParseError::expected(t, Expectation::Boolean));
            }
        };

        Ok(Self {
            span: t.span,
            value,
        })
    }
}

impl Peek for LitBool {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K![true] | K![false])
    }
}

impl ToTokens for LitBool {
    fn to_tokens(&self, _: &mut MacroContext<'_>, stream: &mut TokenStream) {
        stream.push(ast::Token {
            span: self.span,
            kind: if self.value {
                ast::Kind::True
            } else {
                ast::Kind::False
            },
        });
    }
}
