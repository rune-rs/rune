use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Spanned, ToTokens};
use runestick::Id;

/// A module item.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ItemMod>("mod ruins {}");
///
/// let item = testing::roundtrip::<ast::ItemMod>("#[cfg(test)] mod tests {}");
/// assert_eq!(item.attributes.len(), 1);
///
/// let item = testing::roundtrip::<ast::ItemMod>("mod whiskey_bravo { #![allow(dead_code)] fn x() {} }");
/// assert_eq!(item.attributes.len(), 0);
/// assert!(matches!(item.body, ast::ItemModBody::InlineBody(..)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ItemMod {
    /// The id of the module item.
    #[rune(id)]
    pub id: Option<Id>,
    /// The *inner* attributes are applied to the module  `#[cfg(test)] mod tests {  }`
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `mod` item
    #[rune(optional)]
    pub visibility: ast::Visibility,
    /// The `mod` keyword.
    pub mod_: ast::Mod,
    /// The name of the mod.
    pub name: ast::Ident,
    /// The optional body of the module declaration.
    pub body: ItemModBody,
}

impl ItemMod {
    /// Parse a `mod` item with the given attributes
    pub fn parse_with_meta(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        visibility: ast::Visibility,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            id: Default::default(),
            attributes,
            visibility,
            mod_: parser.parse()?,
            name: parser.parse()?,
            body: parser.parse()?,
        })
    }
}

item_parse!(ItemMod, "mod item");

/// An item body.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum ItemModBody {
    /// An empty body terminated by a semicolon.
    EmptyBody(ast::SemiColon),
    /// An inline body.
    InlineBody(ItemInlineBody),
}

impl Parse for ItemModBody {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let t = parser.token_peek_eof()?;

        Ok(match t.kind {
            ast::Kind::Open(ast::Delimiter::Brace) => Self::InlineBody(parser.parse()?),
            _ => Self::EmptyBody(parser.parse()?),
        })
    }
}

/// A module declaration.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ItemInlineBody {
    /// The open brace.
    pub open: ast::OpenBrace,
    /// A nested "file" declaration.
    pub file: Box<ast::File>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl Peek for ItemInlineBody {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        ast::OpenBrace::peek(t1, t2)
    }
}
