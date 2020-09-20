use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek, Spanned, ToTokens};

/// An imported declaration.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ItemUse {
    /// The attributes on use item
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `use` item
    #[rune(iter)]
    pub visibility: Option<ast::Visibility>,
    /// The use token.
    pub use_: ast::Use,
    /// First component in use.
    pub first: ast::Ident,
    /// The rest of the import.
    pub rest: Vec<(ast::Scope, ItemUseComponent)>,
    /// Use items are always terminated by a semi-colon.
    pub semi: ast::SemiColon,
}

impl ItemUse {
    /// Parse a `use` item with the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            visibility: parser.parse()?,
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
        Self::parse_with_attributes(parser, attributes)
    }
}

/// A use component.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub enum ItemUseComponent {
    /// An identifier import.
    Ident(ast::Ident),
    /// A wildcard import.
    Wildcard(ast::Mul),
}

impl Parse for ItemUseComponent {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let t = parser.token_peek_eof()?;

        Ok(match t.kind {
            ast::Kind::Ident(..) => Self::Ident(parser.parse()?),
            ast::Kind::Star => Self::Wildcard(parser.parse()?),
            actual => {
                return Err(ParseError::new(
                    t,
                    ParseErrorKind::ExpectedItemUseImportComponent { actual },
                ));
            }
        })
    }
}

impl Peek for ItemUseComponent {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Ident(..) | ast::Kind::Star)
    }
}
