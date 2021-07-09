use crate::ast;
use crate::{Id, Parse, Peek, Peeker, Spanned, ToTokens};
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
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
pub struct ItemFn {
    /// Opaque identifier for fn item.
    #[rune(id)]
    pub id: Option<Id>,
    /// The attributes for the fn
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `fn` item
    #[rune(optional, meta)]
    pub visibility: ast::Visibility,
    /// The optional `const` keyword.
    #[rune(iter, meta)]
    pub const_token: Option<T![const]>,
    /// The optional `async` keyword.
    #[rune(iter, meta)]
    pub async_token: Option<T![async]>,
    /// The `fn` token.
    pub fn_token: T![fn],
    /// The name of the function.
    pub name: ast::Ident,
    /// The arguments of the function.
    pub args: ast::Parenthesized<ast::FnArg, T![,]>,
    /// The body of the function.
    pub body: ast::Block,
}

impl ItemFn {
    /// Get the descriptive span of this item, e.g. `pub fn foo()` instead of
    /// the span for the whole function declaration, body included.
    pub fn descriptive_span(&self) -> Span {
        if let Some(async_token) = &self.async_token {
            async_token.span().join(self.args.span())
        } else {
            self.fn_token.span().join(self.args.span())
        }
    }

    /// Test if function is an instance fn.
    pub fn is_instance(&self) -> bool {
        matches!(self.args.first(), Some((ast::FnArg::SelfValue(..), _)))
    }
}

item_parse!(Fn, ItemFn, "function item");

impl Peek for ItemFn {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(
            (p.nth(0), p.nth(1)),
            (K![fn], _) | (K![async], K![fn]) | (K![const], K![fn])
        )
    }
}
