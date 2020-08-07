use crate::ast::{Comma, ExprBlock, FnToken, Ident, Parenthesized};
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use stk::unit::Span;

/// A function.
#[derive(Debug, Clone)]
pub struct DeclFn {
    /// The `fn` token.
    pub fn_: FnToken,
    /// The name of the function.
    pub name: Ident,
    /// The arguments of the function.
    pub args: Parenthesized<Ident, Comma>,
    /// The body of the function.
    pub body: ExprBlock,
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
/// # fn main() -> rune::Result<()> {
/// let ParseAll { item, .. } = parse_all::<ast::DeclFn>("fn hello() {}")?;
/// assert_eq!(item.args.items.len(), 0);
///
/// let ParseAll  { source, item } = parse_all::<ast::DeclFn>("fn hello(foo, bar) {}")?;
/// assert_eq!(item.args.items.len(), 2);
/// assert_eq!(item.name.resolve(source)?, "hello");
/// assert_eq!(item.args.items[0].0.resolve(source)?, "foo");
/// assert_eq!(item.args.items[1].0.resolve(source)?, "bar");
/// # Ok(())
/// # }
/// ```
impl Parse for DeclFn {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            fn_: parser.parse()?,
            name: parser.parse()?,
            args: parser.parse()?,
            body: parser.parse()?,
        })
    }
}
