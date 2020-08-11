use crate::ast::Expr;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::token::{Kind, Token};
use crate::traits::{Parse, Peek};
use runestick::unit::Span;
use std::fmt;

/// A binary expression.
#[derive(Debug, Clone)]
pub struct ExprBinary {
    /// The left-hand side of a binary operation.
    pub lhs: Box<Expr>,
    /// The operation to apply.
    pub op: BinOp,
    /// The right-hand side of a binary operation.
    pub rhs: Box<Expr>,
}

impl ExprBinary {
    /// If the expression is empty.
    pub fn produces_nothing(&self) -> bool {
        // Assignments do not produce a value.
        matches!(self.op, BinOp::Assign { .. })
    }

    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.lhs.span().join(self.rhs.span())
    }

    /// Test if the expression is a constant expression.
    pub fn is_const(&self) -> bool {
        self.lhs.is_const() && self.rhs.is_const()
    }
}

/// A binary operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinOp {
    /// Addition.
    Add,
    /// Add assign operation.
    AddAssign,
    /// Subtraction.
    Sub,
    /// Sub assign operation.
    SubAssign,
    /// Division.
    Div,
    /// Div assign operation.
    DivAssign,
    /// Multiplication.
    Mul,
    /// Multiply assign operation.
    MulAssign,
    /// Equality check.
    Eq,
    /// Inequality check.
    Neq,
    /// Greater-than check.
    Gt,
    /// Less-than check.
    Lt,
    /// Greater-than or equal check.
    Gte,
    /// Less-than or equal check.
    Lte,
    /// The dot operator.
    Dot,
    /// The instanceof test.
    Is,
    /// Assign operator.
    Assign,
    /// And `&&` operator.
    And,
    /// Or `||` operator.
    Or,
}

impl BinOp {
    /// Get the precedence for the current operator.
    pub(super) fn precedence(self) -> usize {
        match self {
            Self::Assign => 1,
            Self::AddAssign | Self::SubAssign | Self::MulAssign | Self::DivAssign => 1,
            Self::Or => 2,
            Self::And => 3,
            Self::Eq | Self::Neq | Self::Gt | Self::Lt | Self::Gte | Self::Lte => 4,
            Self::Add | Self::Sub => 5,
            Self::Div | Self::Mul => 6,
            Self::Is => 7,
            Self::Dot => 8,
        }
    }

    /// Test if two operators are associative and can be applied in any order
    /// even if they have the same precedence.
    pub(super) fn is_assoc(self, other: Self) -> bool {
        match (self, other) {
            (Self::Mul, Self::Div) => true,
            (Self::Div, Self::Mul) => true,
            (Self::Add, Self::Sub) => true,
            (Self::Sub, Self::Add) => true,
            (Self::Dot, Self::Dot) => true,
            _ => false,
        }
    }

    /// Convert from a token.
    pub(super) fn from_token(token: Token) -> Option<(BinOp, Token)> {
        let op = match token.kind {
            Kind::Add => Self::Add,
            Kind::AddAssign => Self::AddAssign,
            Kind::Sub => Self::Sub,
            Kind::SubAssign => Self::SubAssign,
            Kind::Div => Self::Div,
            Kind::DivAssign => Self::DivAssign,
            Kind::Mul => Self::Mul,
            Kind::MulAssign => Self::MulAssign,
            Kind::EqEq => Self::Eq,
            Kind::Neq => Self::Neq,
            Kind::Lt => Self::Lt,
            Kind::Gt => Self::Gt,
            Kind::Lte => Self::Lte,
            Kind::Gte => Self::Gte,
            Kind::Dot => Self::Dot,
            Kind::Is => Self::Is,
            Kind::Eq => Self::Assign,
            Kind::And => Self::And,
            Kind::Or => Self::Or,
            _ => return None,
        };

        Some((op, token))
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Add => {
                write!(fmt, "+")?;
            }
            Self::AddAssign => {
                write!(fmt, "+=")?;
            }
            Self::Sub => {
                write!(fmt, "-")?;
            }
            Self::SubAssign => {
                write!(fmt, "-=")?;
            }
            Self::Div => {
                write!(fmt, "/")?;
            }
            Self::DivAssign => {
                write!(fmt, "/=")?;
            }
            Self::Mul => {
                write!(fmt, "*")?;
            }
            Self::MulAssign => {
                write!(fmt, "*=")?;
            }
            Self::Eq => {
                write!(fmt, "==")?;
            }
            Self::Neq => {
                write!(fmt, "!=")?;
            }
            Self::Gt => {
                write!(fmt, ">")?;
            }
            Self::Lt => {
                write!(fmt, "<")?;
            }
            Self::Gte => {
                write!(fmt, ">=")?;
            }
            Self::Lte => {
                write!(fmt, "<=")?;
            }
            Self::Dot => {
                write!(fmt, ".")?;
            }
            Self::Is => {
                write!(fmt, "is")?;
            }
            Self::Assign => {
                write!(fmt, "=")?;
            }
            Self::And => {
                write!(fmt, "&&")?;
            }
            Self::Or => {
                write!(fmt, "||")?;
            }
        }

        Ok(())
    }
}

impl Parse for BinOp {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match Self::from_token(token) {
            Some((op, _)) => op,
            None => {
                return Err(ParseError::ExpectedOperator {
                    span: token.span,
                    actual: token.kind,
                })
            }
        })
    }
}

impl Peek for BinOp {
    fn peek(p1: Option<Token>, _: Option<Token>) -> bool {
        match p1 {
            Some(p1) => match p1.kind {
                Kind::Add => true,
                Kind::Sub => true,
                Kind::Mul => true,
                Kind::Div => true,
                Kind::EqEq => true,
                Kind::Neq => true,
                Kind::Gt => true,
                Kind::Lt => true,
                Kind::Gte => true,
                Kind::Lte => true,
                Kind::Dot => true,
                Kind::Is => true,
                _ => false,
            },
            None => false,
        }
    }
}
