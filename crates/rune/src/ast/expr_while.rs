use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A `while` loop: `while [expr] { ... }`.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprWhile>("while x {}").unwrap();
/// parse_all::<ast::ExprWhile>("'label: while x {}").unwrap();
/// parse_all::<ast::ExprWhile>("#[attr] 'label: while x {}").unwrap();
/// ```
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprWhile {
    /// The attributes for the `while` loop
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// A label for the while loop.
    #[rune(iter)]
    pub label: Option<(ast::Label, ast::Colon)>,
    /// The `while` keyword.
    pub while_: ast::While,
    /// The name of the binding.
    pub condition: ast::Condition,
    /// The body of the while loop.
    pub body: Box<ast::ExprBlock>,
}

impl ExprWhile {
    /// Parse the `while` with the given attributes and label.
    pub fn parse_with_attributes_and_label(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        label: Option<(ast::Label, ast::Colon)>,
    ) -> Result<Self, ParseError> {
        Ok(ExprWhile {
            attributes,
            label,
            while_: parser.parse()?,
            condition: parser.parse()?,
            body: Box::new(parser.parse()?),
        })
    }

    /// Parse the `while` with the given attributes.
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let label = parser.parse()?;
        Self::parse_with_attributes_and_label(parser, attributes, label)
    }
}

impl Parse for ExprWhile {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}
