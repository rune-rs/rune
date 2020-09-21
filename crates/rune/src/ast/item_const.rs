use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A const declaration.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ItemConst {
    /// The *inner* attributes that are applied to the const declaration.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The `const` keyword.
    pub const_token: ast::Const,
    /// The name of the constant.
    pub name: ast::Ident,
    /// The equals token.
    pub eq: ast::Eq,
    /// The optional body of the module declaration.
    pub expr: Box<ast::Expr>,
    /// Terminating semicolon.
    pub semi: ast::SemiColon,
}

impl ItemConst {
    /// Parse a `mod` item with the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            const_token: parser.parse()?,
            name: parser.parse()?,
            eq: parser.parse()?,
            expr: parser.parse()?,
            semi: parser.parse()?,
        })
    }
}

/// Parse a `const` item
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ItemConst>("const value = #{};").unwrap();
/// ```
impl Parse for ItemConst {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}
