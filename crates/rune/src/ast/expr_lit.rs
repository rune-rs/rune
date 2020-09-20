use crate::ast;
use crate::{ParseError, Parser};
use crate::{Spanned, ToTokens};

/// A literal expression.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprLit {
    /// Attributes associated with the literal expression.
    #[rune(iter)]
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
