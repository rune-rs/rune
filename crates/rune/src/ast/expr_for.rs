use crate::ast;
use crate::{ParseError, Parser, Spanned, ToTokens};

/// A `for` loop over an iterator: `for i in [1, 2, 3] {}`.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprFor>("for i in x {}");
/// testing::roundtrip::<ast::ExprFor>("'label: for i in x {}");
/// testing::roundtrip::<ast::ExprFor>("#[attr] 'label: for i in x {}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprFor {
    /// The attributes of the `for` loop
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The label of the loop.
    #[rune(iter)]
    pub label: Option<(ast::Label, T![:])>,
    /// The `for` keyword.
    pub for_token: T![for],
    /// The variable binding.
    /// TODO: should be a pattern when that is supported.
    pub var: ast::Ident,
    /// The `in` keyword.
    pub in_: T![in],
    /// Expression producing the iterator.
    pub iter: ast::Expr,
    /// The body of the loop.
    pub body: Box<ast::Block>,
}

impl ExprFor {
    /// Parse with the given attributes and label.
    pub(crate) fn parse_with_meta(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        label: Option<(ast::Label, T![:])>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            label,
            for_token: parser.parse()?,
            var: parser.parse()?,
            in_: parser.parse()?,
            iter: ast::Expr::parse_without_eager_brace(parser)?,
            body: parser.parse()?,
        })
    }
}

expr_parse!(For, ExprFor, "for loop expression");
