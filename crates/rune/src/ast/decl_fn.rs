use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// A function.
#[derive(Debug, Clone)]
pub struct DeclFn {
    /// The optional `async` keyword.
    pub async_: Option<ast::Async>,
    /// The `fn` token.
    pub fn_: ast::Fn,
    /// The name of the function.
    pub name: ast::Ident,
    /// The arguments of the function.
    pub args: ast::Parenthesized<ast::Ident, ast::Comma>,
    /// The body of the function.
    pub body: ast::ExprBlock,
}

impl DeclFn {
    /// Access the span for the function declaration.
    pub fn span(&self) -> Span {
        self.fn_.span().join(self.body.span())
    }
}

/// Parse implementation for a function.
///
/// # Examples
///
/// ```rust
/// use rune::{ParseAll, parse_all, ast, Resolve as _};
///
/// parse_all::<ast::DeclFn>("async fn hello() {}").unwrap();
/// assert!(parse_all::<ast::DeclFn>("fn async hello() {}").is_err());
///
/// let ParseAll { item, .. } = parse_all::<ast::DeclFn>("fn hello() {}").unwrap();
/// assert_eq!(item.args.items.len(), 0);
///
/// let ParseAll  { source, item } = parse_all::<ast::DeclFn>("fn hello(foo, bar) {}").unwrap();
/// assert_eq!(item.args.items.len(), 2);
/// assert_eq!(item.name.resolve(source).unwrap(), "hello");
/// assert_eq!(item.args.items[0].0.resolve(source).unwrap(), "foo");
/// assert_eq!(item.args.items[1].0.resolve(source).unwrap(), "bar");
/// ```
impl Parse for DeclFn {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            async_: parser.parse()?,
            fn_: parser.parse()?,
            name: parser.parse()?,
            args: parser.parse()?,
            body: parser.parse()?,
        })
    }
}
