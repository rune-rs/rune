use crate::ast;
use crate::{IntoTokens, Parse, ParseError, Parser, Peek, Spanned};
use runestick::Span;

/// Something parenthesized and comma separated `(<T,>*)`.
#[derive(Debug, Clone)]
pub struct Parenthesized<T, S> {
    /// The open parenthesis.
    pub open: ast::OpenParen,
    /// The parenthesized type.
    pub items: Vec<(T, Option<S>)>,
    /// The close parenthesis.
    pub close: ast::CloseParen,
}

impl<T, S> Spanned for Parenthesized<T, S> {
    fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

/// Parse function arguments.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::Parenthesized<ast::Expr, ast::Comma>>("(1, \"two\")").unwrap();
/// parse_all::<ast::Parenthesized<ast::Expr, ast::Comma>>("(1, 2,)").unwrap();
/// parse_all::<ast::Parenthesized<ast::Expr, ast::Comma>>("(1, 2, foo())").unwrap();
/// ```
impl<T, S> Parse for Parenthesized<T, S>
where
    T: Parse,
    S: Peek + Parse,
{
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut items = Vec::new();

        while !parser.peek::<ast::CloseParen>()? {
            let expr = parser.parse()?;
            let sep = parser.parse::<Option<S>>()?;
            let is_end = sep.is_none();
            items.push((expr, sep));

            if is_end {
                break;
            }
        }

        let close = parser.parse()?;
        Ok(Self { open, items, close })
    }
}

impl<T, S> IntoTokens for Parenthesized<T, S>
where
    T: IntoTokens,
    S: IntoTokens,
{
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.open.into_tokens(context, stream);
        self.items.into_tokens(context, stream);
        self.close.into_tokens(context, stream);
    }
}
