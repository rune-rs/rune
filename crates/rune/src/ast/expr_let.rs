use crate::ast::prelude::*;

/// A let expression `let <name> = <expr>`
///
/// # Examples
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::ExprLet>("let x = 1");
/// testing::roundtrip::<ast::ExprLet>("#[attr] let a = f()");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprLet {
    /// The attributes for the let expression
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The `let` keyword.
    pub let_token: T![let],
    /// The name of the binding.
    pub pat: ast::Pat,
    /// The equality keyword.
    pub eq: T![=],
    /// The expression the binding is assigned to.
    pub expr: Box<ast::Expr>,
}

impl ExprLet {
    /// Parse with the given meta.
    pub(crate) fn parse_with_meta(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            let_token: parser.parse()?,
            pat: parser.parse()?,
            eq: parser.parse()?,
            expr: Box::new(ast::Expr::parse_without_eager_brace(parser)?),
        })
    }

    /// Parse a let expression without eager bracing.
    pub(crate) fn parse_without_eager_brace(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            attributes: vec![],
            let_token: parser.parse()?,
            pat: parser.parse()?,
            eq: parser.parse()?,
            expr: Box::new(ast::Expr::parse_without_eager_brace(parser)?),
        })
    }
}

expr_parse!(Let, ExprLet, "let expression");
