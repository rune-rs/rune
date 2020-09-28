use crate::ast;
use crate::ast::Scope;
use crate::{Parse, ParseError, Parser, Peek, Spanned, ToTokens};

/// An imported declaration.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ItemUse {
    /// The attributes on use item.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `use` item.
    #[rune(optional)]
    pub visibility: ast::Visibility,
    /// The use token.
    pub use_: ast::Use,
    /// The path component of the use item.
    pub path: ast::Path,
    /// A trailing `::*` of a wildcard import.
    #[rune(iter)]
    pub wildcard: Option<ast::Mul>,
}

impl ItemUse {
    /// Parse a `use` item with the given attributes
    pub fn parse_with_meta(
        parser: &mut Parser,
        attributes: Vec<ast::Attribute>,
        visibility: ast::Visibility,
    ) -> Result<Self, ParseError> {
        let use_ = parser.parse()?;
        let (path, wildcard) = parser
            .parse::<UsePath>()
            .and_then(UsePath::try_into_path_and_wildcard)?;

        Ok(Self {
            attributes,
            visibility,
            use_,
            path,
            wildcard,
        })
    }

    /// Test if use is a wildcard import by testing if the last
    /// component is a `*` wildcard
    pub fn is_wildcard(&self) -> bool {
        self.wildcard.is_some()
    }
}

/// Parsing an use declaration.
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
/// testing::roundtrip::<ast::ItemUse>("#[macro_use] pub(crate) use foo::bar::baz::*");
/// ```
impl Parse for ItemUse {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        let visibility = parser.parse()?;
        Self::parse_with_meta(parser, attributes, visibility)
    }
}

/// A use component.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
enum ItemUseComponent {
    /// An identifier import.
    PathSegment(ast::PathSegment),
    /// A wildcard import.
    Wildcard(ast::Mul),
}

impl Parse for ItemUseComponent {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        if parser.peek::<ast::PathSegment>()? {
            Ok(Self::PathSegment(parser.parse()?))
        } else if parser.peek::<ast::Mul>()? {
            Ok(Self::Wildcard(parser.parse()?))
        } else {
            let token = parser.token_peek_eof()?;
            Err(ParseError::expected(token, "import component"))
        }
    }
}

impl Peek for ItemUseComponent {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        ast::PathSegment::peek(t1, t2) || ast::Mul::peek(t1, t2)
    }
}

/// A path that may contain wildcard `*` components which cannot
/// be handled by parsing `ast::Path` followed by `Option<ast::Mul>>`
/// due to the parser lookahead believing we are continuing
/// to parse a `Vec<(ast::Scope, ast::PathSegment)>`.
///
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
struct UsePath {
    /// The optional leading `::`.
    #[rune(iter)]
    pub leading_colon: Option<ast::Scope>,
    /// First component in use.
    pub first: ast::PathSegment,
    /// The rest of the import.
    #[rune(iter)]
    pub rest: Vec<(ast::Scope, ItemUseComponent)>,
}

impl UsePath {
    /// Convert use- normal path and an optional trailing wildcard.
    fn try_into_path_and_wildcard(self) -> Result<(ast::Path, Option<ast::Mul>), ParseError> {
        let UsePath {
            leading_colon,
            first,
            rest,
        } = self;

        let mut it = rest.into_iter();
        let last = it.next_back();

        let mut segments = vec![];

        for (scope, comp) in it {
            match comp {
                ItemUseComponent::PathSegment(seg) => segments.push((scope, seg)),
                ItemUseComponent::Wildcard(mul) => {
                    return Err(ParseError::expected(mul.token, "PathSegment"))
                }
            }
        }

        let (trailing, wildcard) = if let Some((scope, comp)) = last {
            match comp {
                ItemUseComponent::PathSegment(seg) => {
                    segments.push((scope, seg));
                    (None, None)
                }
                ItemUseComponent::Wildcard(mul) => (Some(scope), Some(mul)),
            }
        } else {
            (None, None)
        };

        Ok((
            ast::Path {
                leading_colon,
                first,
                rest: segments,
                trailing,
            },
            wildcard,
        ))
    }
}
