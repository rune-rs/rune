use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Spanned, ToTokens};

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
    pub use_token: ast::Use,
    /// Item path.
    pub path: ItemUsePath,
}

item_parse!(ItemUse, "use item");

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
    pub global: Option<ast::Scope>,
    /// The first use component.
    pub first: ItemUseSegment,
    /// Optional segments.
    #[rune(iter)]
    pub segments: Vec<(ast::Scope, ItemUseSegment)>,
    /// The alias of the import.
    #[rune(iter)]
    pub alias: Option<(ast::As, ast::Ident)>,
}

/// A use component.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum ItemUseSegment {
    /// A path segment.
    PathSegment(ast::PathSegment),
    /// A wildcard import.
    Wildcard(ast::Mul),
    /// A grouped import.
    Group(ast::Braced<ast::ItemUsePath, ast::Comma>),
}

impl Peek for ItemUseSegment {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        matches!(
            peek!(t1).kind,
            ast::Kind::Star | ast::Kind::Open(ast::Delimiter::Brace)
        ) || ast::PathSegment::peek(t1, t2)
    }
}

impl Parse for ItemUseSegment {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(match parser.token_peek_eof()?.kind {
            ast::Kind::Star => Self::Wildcard(parser.parse()?),
            ast::Kind::Open(ast::Delimiter::Brace) => Self::Group(parser.parse()?),
            _ => Self::PathSegment(parser.parse()?),
        })
    }
}
