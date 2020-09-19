use crate::ast;
use crate::{Ast, Spanned};
use crate::{ParseError, Parser};

/// A literal expression.
#[derive(Debug, Clone, Ast, Spanned)]
pub struct ExprLit {
    /// Attributes associated with the literal expression.
    #[spanned(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The literal in the expression.
    pub lit: ast::Lit,
}

impl ExprLit {
    /// Test if the literal expression is constant.
    pub fn is_const(&self) -> bool {
        self.lit.is_const()
    }

    /// Parse the literal expression with attributes.
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
