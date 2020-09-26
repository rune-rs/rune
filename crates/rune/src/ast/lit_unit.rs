use crate::ast;
use crate::{Parse, Peek, Spanned, ToTokens};

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
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        matches! {
            (peek!(t1).kind, peek!(t2).kind),
            (
                ast::Kind::Open(ast::Delimiter::Parenthesis),
                ast::Kind::Close(ast::Delimiter::Parenthesis),
            )
        }
    }
}
