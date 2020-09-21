use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A `for` loop over an iterator: `for i in [1, 2, 3] {}`.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprFor>("for i in x {}").unwrap();
/// parse_all::<ast::ExprFor>("'label: for i in x {}").unwrap();
/// parse_all::<ast::ExprFor>("#[attr] 'label: for i in x {}").unwrap();
/// ```
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprFor {
    /// The attributes of the `for` loop
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The label of the loop.
    #[rune(iter)]
    pub label: Option<(ast::Label, ast::Colon)>,
    /// The `for` keyword.
    pub for_: ast::For,
    /// The variable binding.
    /// TODO: should be a pattern when that is supported.
    pub var: ast::Ident,
    /// The `in` keyword.
    pub in_: ast::In,
    /// Expression producing the iterator.
    pub iter: Box<ast::Expr>,
    /// The body of the loop.
    pub body: Box<ast::ExprBlock>,
}

impl ExprFor {
    /// Parse with the given attributes and label.
    pub fn parse_with_attributes_and_label(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        label: Option<(ast::Label, ast::Colon)>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            label,
            for_: parser.parse()?,
            var: parser.parse()?,
            in_: parser.parse()?,
            iter: Box::new(ast::Expr::parse_without_eager_brace(parser)?),
            body: Box::new(parser.parse()?),
        })
    }

    /// Parse the `for` loop with the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let label = if parser.peek::<ast::Label>()? {
            Some((parser.parse()?, parser.parse()?))
        } else {
            None
        };
        Self::parse_with_attributes_and_label(parser, attributes, label)
    }
}

impl Parse for ExprFor {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}
