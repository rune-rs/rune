use crate::ast;
use crate::{Ast, Parse, ParseError, Parser, Peek, Spanned};
use runestick::Span;

/// A module declaration.
#[derive(Debug, Clone, Ast)]
pub struct ItemMod {
    /// The *inner* attributes are applied to the module  `#[cfg(test)] mod tests {  }`
    pub attributes: Vec<ast::Attribute>,
    /// The `mod` keyword.
    pub mod_: ast::Mod,
    /// The name of the mod.
    pub name: ast::Ident,
    /// The optional body of the module declaration.
    pub body: ItemModBody,
}

impl ItemMod {
    /// Parse a `mod` item with the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            mod_: parser.parse()?,
            name: parser.parse()?,
            body: parser.parse()?,
        })
    }
}

impl Spanned for ItemMod {
    fn span(&self) -> Span {
        if let Some(first) = self.attributes.first() {
            first.span().join(self.body.span())
        } else {
            self.mod_.span().join(self.body.span())
        }
    }
}

/// Parse a `mod` item
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, ParseError};
///
/// parse_all::<ast::ItemMod>("mod ruins {}").unwrap();
///
/// let item = parse_all::<ast::ItemMod>("#[cfg(test)] mod tests {}").unwrap();
/// assert_eq!(item.attributes.len(), 1);
///
/// let item = parse_all::<ast::ItemMod>("mod whiskey_bravo { #![allow(dead_code)] fn x() {} }").unwrap();
/// assert_eq!(item.attributes.len(), 0);
///
/// if let ast::ItemModBody::InlineBody(body) = &item.body {
///     assert_eq!(body.file.attributes.len(), 1);
/// } else {
///     panic!("module body was not the ItemModBody::InlineBody variant");
/// }
///
/// ```
impl Parse for ItemMod {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}

/// An item body.
#[derive(Debug, Clone, Ast)]
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
#[derive(Debug, Clone, Ast, Parse)]
pub struct ItemInlineBody {
    /// The open brace.
    pub open: ast::OpenBrace,
    /// A nested "file" declaration.
    pub file: Box<ast::File>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl ItemInlineBody {
    /// The span of the body.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

impl Peek for ItemInlineBody {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        ast::OpenBrace::peek(t1, t2)
    }
}
