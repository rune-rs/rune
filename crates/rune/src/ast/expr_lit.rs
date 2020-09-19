use crate::ast;
use crate::Spanned;
use crate::{ParseError, Parser};

/// A literal expression.
#[derive(Debug, Clone)]
pub struct ExprLit {
    /// Attributes associated with the literal expression.
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

into_tokens!(ExprLit { attributes, lit });

impl Spanned for ExprLit {
    fn span(&self) -> runestick::Span {
        if let Some(first) = self.attributes.first() {
            first.span().join(self.lit.span())
        } else {
            self.lit.span()
        }
    }
}
