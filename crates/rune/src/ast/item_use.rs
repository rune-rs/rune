use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Peeker, Spanned, ToTokens};

/// A use item.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ItemUse>("use foo");
/// testing::roundtrip::<ast::ItemUse>("use foo::bar");
/// testing::roundtrip::<ast::ItemUse>("use foo::bar::baz");
/// testing::roundtrip::<ast::ItemUse>("#[macro_use] use foo::bar::baz");
/// testing::roundtrip::<ast::ItemUse>("#[macro_use] pub(crate) use foo::bar::baz");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
pub struct ItemUse {
    /// The attributes on use item
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `use` item
    #[rune(optional, meta)]
    pub visibility: ast::Visibility,
    /// The use token.
    pub use_token: T![use],
    /// Item path.
    pub path: ItemUsePath,
}

item_parse!(Use, ItemUse, "use item");

/// A single use declaration path, like `foo::bar::{baz::*, biz}`.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ItemUsePath>("crate::foo");
/// testing::roundtrip::<ast::ItemUsePath>("foo::bar");
/// testing::roundtrip::<ast::ItemUsePath>("foo::bar::{baz::*, biz}");
/// testing::roundtrip::<ast::ItemUsePath>("{*, bar::*}");
/// testing::roundtrip::<ast::ItemUsePath>("::{*, bar::*}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
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
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
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
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K![*] => Self::Wildcard(p.parse()?),
            K!['{'] => Self::Group(p.parse()?),
            _ => Self::PathSegment(p.parse()?),
        })
    }
}
