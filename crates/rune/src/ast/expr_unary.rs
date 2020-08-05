use crate::ast::expr::NoIndex;
use crate::ast::Expr;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::token::{Kind, Token};
use crate::traits::Parse;
use st::unit::Span;
use std::fmt;

/// A unary expression.
#[derive(Debug, Clone)]
pub struct ExprUnary {
    /// The operation to apply.
    pub op: UnaryOp,
    /// The expression of the operation.
    pub expr: Box<Expr>,
}

impl ExprUnary {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.op.span().join(self.expr.span())
    }
}

impl Parse for ExprUnary {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            op: parser.parse()?,
            expr: Box::new(Expr::parse_primary(parser, NoIndex(false))?),
        })
    }
}

/// A unary operation.
#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    /// Not `!<thing>`.
    Not {
        /// Token associated with operator.
        token: Token,
    },
    /// Reference `&<thing>`.
    Ref {
        /// Token associated with operator.
        token: Token,
    },
    /// Dereference `*<thing>`.
    Deref {
        /// Token associated with operator.
        token: Token,
    },
}

impl UnaryOp {
    /// Access the span of the unary operator.
    pub fn span(&self) -> Span {
        match self {
            Self::Not { token } => token.span,
            Self::Ref { token } => token.span,
            Self::Deref { token } => token.span,
        }
    }
}

impl Parse for UnaryOp {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match token.kind {
            Kind::Not => Self::Not { token },
            Kind::Ampersand => Self::Ref { token },
            Kind::Star => Self::Deref { token },
            actual => {
                return Err(ParseError::ExpectedUnaryOperator {
                    span: token.span,
                    actual,
                })
            }
        })
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Not { .. } => write!(fmt, "!")?,
            Self::Ref { .. } => write!(fmt, "&")?,
            Self::Deref { .. } => write!(fmt, "*")?,
        }

        Ok(())
    }
}
