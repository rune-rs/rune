use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek, Spanned, ToTokens};

/// An imported declaration.
#[derive(Debug, Clone, ToTokens, Spanned)]
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
    pub use_: ast::Use,
    /// First component in use.
    pub first: ast::PathSegment,
    /// The rest of the import.
    pub rest: Vec<(ast::Scope, ItemUseComponent)>,
    /// Use items are always terminated by a semi-colon.
    pub semi: ast::SemiColon,
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
            use_: parser.parse()?,
            first: parser.parse()?,
            rest: parser.parse()?,
            semi: parser.parse()?,
        })
    }
}

/// Parsing an use declaration.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ItemUse>("use foo;").unwrap();
/// parse_all::<ast::ItemUse>("use foo::bar;").unwrap();
/// parse_all::<ast::ItemUse>("use foo::bar::baz;").unwrap();
/// parse_all::<ast::ItemUse>("#[macro_use] use foo::bar::baz;").unwrap();
/// parse_all::<ast::ItemUse>("#[macro_use] pub(crate) use foo::bar::baz;").unwrap();
/// ```
impl Parse for ItemUse {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        let visibility = parser.parse()?;
        Self::parse_with_meta(parser, attributes, visibility)
    }
}

/// A use component.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub enum ItemUseComponent {
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
            Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedItemUseImportComponent { actual: token.kind },
            ))
        }
    }
}

impl Peek for ItemUseComponent {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        ast::PathSegment::peek(t1, t2) || ast::Mul::peek(t1, t2)
    }
}
