use crate::ast;
use crate::{Parse, Peek, Peeker, Spanned, ToTokens};

/// The unit literal `()`.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitUnit>("()");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct LitUnit {
    /// The open parenthesis.
    pub open: ast::OpenParen,
    /// The close parenthesis.
    pub close: ast::CloseParen,
}

impl Peek for LitUnit {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!((p.nth(0), p.nth(1)), (K!['('], K![')']))
    }
}
