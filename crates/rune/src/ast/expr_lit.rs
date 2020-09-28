use crate::ast;
use crate::{ParseError, Parser, Spanned, ToTokens};

/// A literal expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprLit {
    /// Attributes associated with the literal expression.
    #[rune(iter, attributes)]
    pub attributes: Vec<ast::Attribute>,
    /// The literal in the expression.
    pub lit: ast::Lit,
}

impl ExprLit {
    /// Parse with the given attributes.
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            lit: parser.parse()?,
        })
    }
}

expr_parse!(ExprLit, "literal expression");
