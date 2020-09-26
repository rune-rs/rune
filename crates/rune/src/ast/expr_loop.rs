use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A `loop` expression: `loop { ... }`.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprLoop>("loop {}");
/// testing::roundtrip::<ast::ExprLoop>("loop { 1; }");
/// testing::roundtrip::<ast::ExprLoop>("'label: loop {1;}");
/// testing::roundtrip::<ast::ExprLoop>("#[attr] 'label: loop {x();}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprLoop {
    /// The attributes for the `loop`
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// A label followed by a colon.
    #[rune(iter)]
    pub label: Option<(ast::Label, ast::Colon)>,
    /// The `loop` keyword.
    pub loop_: ast::Loop,
    /// The body of the loop.
    pub body: Box<ast::ExprBlock>,
}

impl ExprLoop {
    /// Parse the `loop` the given attributes and label.
    pub fn parse_with_attributes_and_label(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        label: Option<(ast::Label, ast::Colon)>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            label,
            loop_: parser.parse()?,
            body: Box::new(parser.parse()?),
        })
    }

    /// Parse the `loop` with the given attributes.
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let label = parser.parse()?;
        Self::parse_with_attributes_and_label(parser, attributes, label)
    }
}

impl Parse for ExprLoop {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}
