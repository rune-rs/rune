use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ItemUse>("use foo");
    rt::<ast::ItemUse>("use foo::bar");
    rt::<ast::ItemUse>("use foo::bar::baz");
    rt::<ast::ItemUse>("#[macro_use] use foo::bar::baz");
    rt::<ast::ItemUse>("#[macro_use] pub(crate) use foo::bar::baz");

    rt::<ast::ItemUsePath>("crate::foo");
    rt::<ast::ItemUsePath>("foo::bar");
    rt::<ast::ItemUsePath>("foo::bar::{baz::*, biz}");
    rt::<ast::ItemUsePath>("{*, bar::*}");
    rt::<ast::ItemUsePath>("::{*, bar::*}");
}

/// A `use` item.
///
/// * `use <path>`
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ItemUse {
    /// The attributes on use item
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `use` item
    #[rune(option, meta)]
    pub visibility: ast::Visibility,
    /// The use token.
    pub use_token: T![use],
    /// Item path.
    pub path: ItemUsePath,
}

item_parse!(Use, ItemUse, "use item");

/// A single use declaration path.
///
/// * `foo::bar::{baz::*, biz}`.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ItemUsePath {
    /// Global prefix.
    #[rune(iter)]
    pub global: Option<T![::]>,
    /// The first use component.
    pub first: ItemUseSegment,
    /// Optional segments.
    #[rune(iter)]
    pub segments: Vec<(T![::], ItemUseSegment)>,
    /// The alias of the import.
    #[rune(iter)]
    pub alias: Option<(T![as], ast::Ident)>,
}

/// A use component.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum ItemUseSegment {
    /// A path segment.
    PathSegment(ast::PathSegment),
    /// A wildcard import.
    Wildcard(T![*]),
    /// A grouped import.
    Group(ast::Braced<ast::ItemUsePath, T![,]>),
}

impl Peek for ItemUseSegment {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K![*] | K!['[']) || ast::PathSegment::peek(p)
    }
}

impl Parse for ItemUseSegment {
    fn parse(p: &mut Parser) -> Result<Self> {
        Ok(match p.nth(0)? {
            K![*] => Self::Wildcard(p.parse()?),
            K!['{'] => Self::Group(p.parse()?),
            _ => Self::PathSegment(p.parse()?),
        })
    }
}
