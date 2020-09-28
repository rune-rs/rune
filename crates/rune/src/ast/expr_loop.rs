use crate::ast;
use crate::{ParseError, Parser, Spanned, ToTokens};

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
    pub body: Box<ast::Block>,
}

impl ExprLoop {
    /// Parse the `loop` the given attributes and label.
    pub(crate) fn parse_with_meta(
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
}

expr_parse!(ExprLoop, "loop expression");
