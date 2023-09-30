use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprLet>("let x = 1");
    rt::<ast::ExprLet>("#[attr] let a = f()");
}

/// A let expression.
///
/// * `let <name> = <expr>`
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprLet {
    /// The attributes for the let expression
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The `let` token.
    pub let_token: T![let],
    /// The `mut` token.
    #[rune(iter)]
    pub mut_token: Option<T![mut]>,
    /// The name of the binding.
    pub pat: ast::Pat,
    /// The equality token.
    pub eq: T![=],
    /// The expression the binding is assigned to.
    pub expr: Box<ast::Expr>,
}

impl ExprLet {
    /// Parse with the given meta.
    pub(crate) fn parse_with_meta(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self> {
        Ok(Self {
            attributes,
            let_token: parser.parse()?,
            mut_token: parser.parse()?,
            pat: parser.parse()?,
            eq: parser.parse()?,
            expr: Box::try_new(ast::Expr::parse_without_eager_brace(parser)?)?,
        })
    }

    /// Parse a let expression without eager bracing.
    pub(crate) fn parse_without_eager_brace(parser: &mut Parser) -> Result<Self> {
        Ok(Self {
            attributes: Vec::new(),
            let_token: parser.parse()?,
            mut_token: parser.parse()?,
            pat: parser.parse()?,
            eq: parser.parse()?,
            expr: Box::try_new(ast::Expr::parse_without_eager_brace(parser)?)?,
        })
    }
}

expr_parse!(Let, ExprLet, "let expression");
