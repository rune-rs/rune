use crate::ast;
use crate::{Id, ParseError, Parser, Spanned, ToTokens};

/// A const declaration.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ItemConst>("const value = #{}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ItemConst {
    /// Opaque identifier for the constant.
    #[rune(id)]
    pub id: Id,
    /// The *inner* attributes that are applied to the const declaration.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the const.
    #[rune(optional)]
    pub visibility: ast::Visibility,
    /// The `const` keyword.
    pub const_token: ast::Const,
    /// The name of the constant.
    pub name: ast::Ident,
    /// The equals token.
    pub eq: ast::Eq,
    /// The optional body of the module declaration.
    pub expr: Box<ast::Expr>,
}

impl ItemConst {
    /// Parse a `const` item with the given attributes
    pub fn parse_with_meta(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        visibility: ast::Visibility,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            id: Default::default(),
            attributes,
            visibility,
            const_token: parser.parse()?,
            name: parser.parse()?,
            eq: parser.parse()?,
            expr: parser.parse()?,
        })
    }
}

item_parse!(ItemConst, "constant item");
