use crate::ast;
use crate::ast::{Kind, Token};
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::{Parse, Peek};
use runestick::Span;

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
    pub args: ast::Parenthesized<ast::FnArg, ast::Comma>,
    /// The body of the function.
    pub body: ast::ExprBlock,
}

impl DeclFn {
    /// Get the identifying span for this function.
    pub fn item_span(&self) -> Span {
        if let Some(async_) = &self.async_ {
            async_.span().join(self.args.span())
        } else {
            self.fn_.span().join(self.args.span())
        }
    }

    /// Access the span for the function declaration.
    pub fn span(&self) -> Span {
        if let Some(async_) = &self.async_ {
            async_.span().join(self.body.span())
        } else {
            self.fn_.span().join(self.body.span())
        }
    }

    /// Test if function is an instance fn.
    pub fn is_instance(&self) -> bool {
        match self.args.items.iter().next() {
            Some((ast::FnArg::Self_(..), _)) => true,
            _ => false,
        }
    }
}

impl Peek for DeclFn {
    fn peek(t1: Option<Token>, _: Option<Token>) -> bool {
        matches!(t1, Some(Token { kind: Kind::Fn, .. }))
    }
}

/// Parse implementation for a function.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::DeclFn>("async fn hello() {}").unwrap();
/// assert!(parse_all::<ast::DeclFn>("fn async hello() {}").is_err());
///
/// let item = parse_all::<ast::DeclFn>("fn hello() {}").unwrap();
/// assert_eq!(item.args.items.len(), 0);
///
/// let item = parse_all::<ast::DeclFn>("fn hello(foo, bar) {}").unwrap();
/// assert_eq!(item.args.items.len(), 2);
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
