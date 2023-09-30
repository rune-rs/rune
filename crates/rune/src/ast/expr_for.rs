use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprFor>("for i in x {}");
    rt::<ast::ExprFor>("for (a, _) in x {}");
    rt::<ast::ExprFor>("'label: for i in x {}");
    rt::<ast::ExprFor>("#[attr] 'label: for i in x {}");
}

/// A `for` loop over an iterator.
///
/// * `for <pat> in <expr> <block>`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprFor {
    /// The attributes of the `for` loop
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The label of the loop.
    #[rune(iter)]
    pub label: Option<(ast::Label, T![:])>,
    /// The `for` keyword.
    pub for_token: T![for],
    /// The pattern binding to use.
    /// Non-trivial pattern bindings will panic if the value doesn't match.
    pub binding: ast::Pat,
    /// The `in` keyword.
    pub in_: T![in],
    /// Expression producing the iterator.
    pub iter: Box<ast::Expr>,
    /// The body of the loop.
    pub body: Box<ast::Block>,
}

impl ExprFor {
    /// Parse with the given attributes and label.
    pub(crate) fn parse_with_meta(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        label: Option<(ast::Label, T![:])>,
    ) -> Result<Self> {
        Ok(Self {
            attributes,
            label,
            for_token: parser.parse()?,
            binding: parser.parse()?,
            in_: parser.parse()?,
            iter: Box::try_new(ast::Expr::parse_without_eager_brace(parser)?)?,
            body: parser.parse()?,
        })
    }
}

expr_parse!(For, ExprFor, "for loop expression");
