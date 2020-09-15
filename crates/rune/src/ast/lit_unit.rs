use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Spanned};
use runestick::Span;

/// The unit literal `()`.
#[derive(Debug, Clone)]
pub struct LitUnit {
    /// The open parenthesis.
    pub open: ast::OpenParen,
    /// The close parenthesis.
    pub close: ast::CloseParen,
}

into_tokens!(LitUnit { open, close });

impl Spanned for LitUnit {
    fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

/// Parsing a unit literal
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::LitUnit>("()").unwrap();
/// ```
impl Parse for LitUnit {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            open: parser.parse()?,
            close: parser.parse()?,
        })
    }
}

impl Peek for LitUnit {
    fn peek(p1: Option<ast::Token>, p2: Option<ast::Token>) -> bool {
        let (p1, p2) = match (p1, p2) {
            (Some(p1), Some(p2)) => (p1, p2),
            _ => return false,
        };

        matches! {
            (p1.kind, p2.kind),
            (
                ast::Kind::Open(ast::Delimiter::Parenthesis),
                ast::Kind::Close(ast::Delimiter::Parenthesis),
            )
        }
    }
}
