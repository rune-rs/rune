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
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ItemUse {
    /// The attributes on use item
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `use` item
    #[rune(optional)]
    pub visibility: ast::Visibility,
    /// The optional leading `::`
    #[rune(iter)]
    pub leading_colon: Option<ast::Scope>,
    /// The use token.
    pub use_token: ast::Use,
    /// Item path.
    pub path: ItemUsePath,
}

impl ItemUse {
    /// Parse a `use` item with the given attributes
    pub fn parse_with_meta(
        parser: &mut Parser,
        attributes: Vec<ast::Attribute>,
        visibility: ast::Visibility,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            visibility,
            leading_colon: parser.parse()?,
            use_token: parser.parse()?,
            path: parser.parse()?,
        })
    }
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
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ItemUsePath {
    /// First component in use.
    pub first: ast::PathSegment,
    /// The middle part of the import.
    #[rune(iter)]
    pub middle: Vec<(ast::Scope, ast::PathSegment)>,
    /// The optional last group component.
    #[rune(iter)]
    pub last: Option<(ast::Scope, ItemUseComponent)>,
    /// The alias of the import.
    #[rune(iter)]
    pub alias: Option<(ast::As, ast::Ident)>,
}

impl Parse for ItemUsePath {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let first = parser.parse()?;
        let mut middle = Vec::new();
        let mut last = None;

        while parser.peek::<ast::Scope>()? {
            let scope = parser.parse::<ast::Scope>()?;

            if parser.peek::<ast::PathSegment>()? {
                middle.push((scope, parser.parse()?));
            } else {
                last = Some((scope, parser.parse()?));
                break;
            }
        }

        Ok(Self {
            first,
            middle,
            last,
            alias: parser.parse()?,
        })
    }
}

/// A use component.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum ItemUseComponent {
    /// A wildcard import.
    Wildcard(ast::Mul),
    /// A grouped import.
    Group(ast::Braced<ast::ItemUsePath, ast::Comma>),
}

impl Peek for ItemUseComponent {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        matches!(
            peek!(t1).kind,
            ast::Kind::Star | ast::Kind::Open(ast::Delimiter::Brace)
        )
    }
}

impl Parse for ItemUseComponent {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(if parser.peek::<ast::Mul>()? {
            Self::Wildcard(parser.parse()?)
        } else {
            Self::Group(parser.parse()?)
        })
    }
}
