use crate::ast;
use crate::{Id, ParseError, Parser, Peek, Spanned, ToTokens};
use runestick::Span;

/// A function item.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast, parse_all};
///
/// testing::roundtrip::<ast::ItemFn>("async fn hello() {}");
/// assert!(parse_all::<ast::ItemFn>("fn async hello() {}").is_err());
///
/// let item = testing::roundtrip::<ast::ItemFn>("fn hello() {}");
/// assert_eq!(item.args.len(), 0);
///
/// let item = testing::roundtrip::<ast::ItemFn>("fn hello(foo, bar) {}");
/// assert_eq!(item.args.len(), 2);
///
/// testing::roundtrip::<ast::ItemFn>("pub fn hello(foo, bar) {}");
/// testing::roundtrip::<ast::ItemFn>("pub async fn hello(foo, bar) {}");
/// testing::roundtrip::<ast::ItemFn>("#[inline] fn hello(foo, bar) {}");
///
/// let item = testing::roundtrip::<ast::ItemFn>("#[inline] pub async fn hello(foo, bar) {}");
/// assert!(matches!(item.visibility, ast::Visibility::Public(..)));
///
/// assert_eq!(item.args.len(), 2);
/// assert_eq!(item.attributes.len(), 1);
/// assert!(item.async_token.is_some());
/// assert!(item.const_token.is_none());
///
/// let item = testing::roundtrip::<ast::ItemFn>("#[inline] pub const fn hello(foo, bar) {}");
/// assert!(matches!(item.visibility, ast::Visibility::Public(..)));
///
/// assert_eq!(item.args.len(), 2);
/// assert_eq!(item.attributes.len(), 1);
/// assert!(item.async_token.is_none());
/// assert!(item.const_token.is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ItemFn {
    /// Opaque identifier for fn item.
    #[rune(id)]
    pub id: Id,
    /// The attributes for the fn
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `fn` item
    #[rune(optional)]
    pub visibility: ast::Visibility,
    /// The optional `const` keyword.
    #[rune(iter)]
    pub const_token: Option<ast::Const>,
    /// The optional `async` keyword.
    #[rune(iter)]
    pub async_token: Option<ast::Async>,
    /// The `fn` token.
    pub fn_: ast::Fn,
    /// The name of the function.
    pub name: ast::Ident,
    /// The arguments of the function.
    pub args: ast::Parenthesized<ast::FnArg, ast::Comma>,
    /// The body of the function.
    pub body: ast::Block,
}

impl ItemFn {
    /// Get the identifying span for this function.
    pub fn item_span(&self) -> Span {
        if let Some(async_token) = &self.async_token {
            async_token.span().join(self.args.span())
        } else {
            self.fn_.span().join(self.args.span())
        }
    }

    /// Test if function is an instance fn.
    pub fn is_instance(&self) -> bool {
        matches!(self.args.first(), Some((ast::FnArg::Self_(..), _)))
    }

    /// Parse a `fn` item with the given meta
    pub fn parse_with_meta(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        visibility: ast::Visibility,
        const_token: Option<ast::Const>,
        async_token: Option<ast::Async>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            id: Default::default(),
            attributes,
            visibility,
            const_token,
            async_token,
            fn_: parser.parse()?,
            name: parser.parse()?,
            args: parser.parse()?,
            body: parser.parse()?,
        })
    }
}

item_parse!(ItemFn, "function item");

impl Peek for ItemFn {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        matches!(
            peek!(t1).kind,
            ast::Kind::Fn | ast::Kind::Async | ast::Kind::Const
        )
    }
}
