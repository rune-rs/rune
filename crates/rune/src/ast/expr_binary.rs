use crate::ast;
use crate::{Peek, Spanned};
use runestick::Span;
use std::fmt;

/// A binary expression.
#[derive(Debug, Clone)]
pub struct ExprBinary {
    /// The left-hand side of a binary operation.
    pub lhs: Box<ast::Expr>,
    /// Token associated with operator.
    pub t1: ast::Token,
    /// Token associated with optional second part of operator.
    pub t2: Option<ast::Token>,
    /// The right-hand side of a binary operation.
    pub rhs: Box<ast::Expr>,
    /// The operation to apply.
    pub op: BinOp,
}

into_tokens!(ExprBinary { lhs, t1, t2, rhs });

impl ExprBinary {
    /// If the expression is empty.
    pub fn produces_nothing(&self) -> bool {
        // Assignments do not produce a value.
        matches!(self.op, BinOp::Assign { .. })
    }

    /// Test if the expression is a constant expression.
    pub fn is_const(&self) -> bool {
        self.lhs.is_const() && self.rhs.is_const()
    }
}

impl Spanned for ExprBinary {
    fn span(&self) -> Span {
        self.lhs.span().join(self.rhs.span())
    }
}

/// A binary operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinOp {
    /// Addition `a + b`.
    Add,
    /// Subtraction `a - b`.
    Sub,
    /// Division `a / b`.
    Div,
    /// Multiplication `a * b`.
    Mul,
    /// Remainder operator `a % b`.
    Rem,
    /// Equality check `a == b`.
    Eq,
    /// Inequality check `a != b`.
    Neq,
    /// Greater-than check `a > b`.
    Gt,
    /// Less-than check `a < b`.
    Lt,
    /// Greater-than or equal check `a >= b`.
    Gte,
    /// Less-than or equal check `a <= b`.
    Lte,
    /// Instance of test `a is b`.
    Is,
    /// Negated instance of test `a is not b`.
    IsNot,
    /// Lazy and operator `&&`.
    And,
    /// Lazy or operator `||`.
    Or,
    /// Bitwise left shift operator `a << b`.
    Shl,
    /// Bitwise right shift operator `a >> b`.
    Shr,
    /// Bitwise and operator `a & b`.
    BitAnd,
    /// Bitwise xor operator `a ^ b`.
    BitXor,
    /// Bitwise or operator `a | b`.
    BitOr,
    /// Assign operator `a = b`.
    Assign,
    /// Add assign `a += b`.
    AddAssign,
    /// Sub assign `a -= b`.
    SubAssign,
    /// Multiply assign operation `a *= b`.
    MulAssign,
    /// Div assign `a /= b`.
    DivAssign,
    /// Remainder assign `a %= b`.
    RemAssign,
    /// Bitwise and assign `a &= b`.
    BitAndAssign,
    /// Bitwise xor assign `a ^= b`.
    BitXorAssign,
    /// Bitwise or assign `a |= b`.
    BitOrAssign,
    /// Left shift assign `a <<= b`.
    ShlAssign,
    /// Right shift assign `a >>= b`.
    ShrAssign,
}

impl BinOp {
    /// Test if operator is an assign operator.
    pub fn is_assign(self) -> bool {
        match self {
            Self::Assign => true,
            Self::AddAssign => true,
            Self::SubAssign => true,
            Self::MulAssign => true,
            Self::DivAssign => true,
            Self::RemAssign => true,
            Self::BitAndAssign => true,
            Self::BitXorAssign => true,
            Self::BitOrAssign => true,
            Self::ShlAssign => true,
            Self::ShrAssign => true,
            _ => false,
        }
    }

    /// Get the precedence for the current operator.
    pub(super) fn precedence(self) -> usize {
        // NB: Rules from: https://doc.rust-lang.org/reference/expressions.html#expression-precedence
        match self {
            Self::Is | Self::IsNot => 11,
            Self::Mul | Self::Div | Self::Rem => 10,
            Self::Add | Self::Sub => 9,
            Self::Shl | Self::Shr => 8,
            Self::BitAnd => 7,
            Self::BitXor => 6,
            Self::BitOr => 5,
            Self::Eq | Self::Neq | Self::Lt | Self::Gt | Self::Lte | Self::Gte => 4,
            Self::And => 3,
            Self::Or => 2,
            // assign operators
            _ => 1,
        }
    }

    /// Test if operator is left associative.
    pub(super) fn is_assoc(self) -> bool {
        match self {
            Self::Mul => true,
            Self::Div => true,
            Self::Add => true,
            Self::Sub => true,
            Self::Or => true,
            Self::And => true,
            _ => false,
        }
    }

    /// Convert from a token.
    pub(super) fn from_token(
        (t1, t2): (ast::Token, Option<ast::Token>),
    ) -> Option<(BinOp, ast::Token, Option<ast::Token>)> {
        let op = match t1.kind {
            ast::Kind::Plus => Self::Add,
            ast::Kind::Dash => Self::Sub,
            ast::Kind::Div => Self::Div,
            ast::Kind::Star => Self::Mul,
            ast::Kind::Perc => Self::Rem,
            ast::Kind::EqEq => Self::Eq,
            ast::Kind::BangEq => Self::Neq,
            ast::Kind::Lt => Self::Lt,
            ast::Kind::Gt => Self::Gt,
            ast::Kind::LtEq => Self::Lte,
            ast::Kind::GtEq => Self::Gte,
            ast::Kind::Is => {
                if let Some(t2) = t2 {
                    if let ast::Kind::Not = t2.kind {
                        return Some((Self::IsNot, t1, Some(t2)));
                    }
                }

                Self::Is
            }
            ast::Kind::Eq => Self::Assign,
            ast::Kind::AmpAmp => Self::And,
            ast::Kind::PipePipe => Self::Or,
            ast::Kind::LtLt => Self::Shl,
            ast::Kind::GtGt => Self::Shr,
            ast::Kind::Amp => Self::BitAnd,
            ast::Kind::Caret => Self::BitXor,
            ast::Kind::Pipe => Self::BitOr,
            ast::Kind::PlusEq => Self::AddAssign,
            ast::Kind::DashEq => Self::SubAssign,
            ast::Kind::StarEq => Self::MulAssign,
            ast::Kind::SlashEq => Self::DivAssign,
            ast::Kind::PercEq => Self::RemAssign,
            ast::Kind::AmpEq => Self::BitAndAssign,
            ast::Kind::CaretEq => Self::BitXorAssign,
            ast::Kind::PipeEq => Self::BitOrAssign,
            ast::Kind::LtLtEq => Self::ShlAssign,
            ast::Kind::GtGtEq => Self::ShrAssign,
            _ => return None,
        };

        Some((op, t1, None))
    }

    /// Get how many tokens to advance for this operator.
    pub(crate) fn advance(&self) -> usize {
        match self {
            Self::IsNot => 2,
            _ => 1,
        }
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Div => write!(f, "/"),
            Self::Mul => write!(f, "*"),
            Self::Rem => write!(f, "%"),
            Self::Eq => write!(f, "=="),
            Self::Neq => write!(f, "!="),
            Self::Gt => write!(f, ">"),
            Self::Lt => write!(f, "<"),
            Self::Gte => write!(f, ">="),
            Self::Lte => write!(f, "<="),
            Self::Is => write!(f, "is"),
            Self::IsNot => write!(f, "is not"),
            Self::Assign => write!(f, "="),
            Self::And => write!(f, "&&"),
            Self::Or => write!(f, "||"),
            Self::Shl => write!(f, "<<"),
            Self::Shr => write!(f, ">>"),
            Self::BitAnd => write!(f, "&"),
            Self::BitXor => write!(f, "^"),
            Self::BitOr => write!(f, "|"),
            Self::AddAssign => write!(f, "+="),
            Self::SubAssign => write!(f, "-="),
            Self::DivAssign => write!(f, "/="),
            Self::MulAssign => write!(f, "*="),
            Self::BitAndAssign => write!(f, "&="),
            Self::BitXorAssign => write!(f, "^="),
            Self::BitOrAssign => write!(f, "|="),
            Self::RemAssign => write!(f, "%="),
            Self::ShlAssign => write!(f, "<<="),
            Self::ShrAssign => write!(f, ">>="),
        }
    }
}

impl Peek for BinOp {
    fn peek(p1: Option<ast::Token>, p2: Option<ast::Token>) -> bool {
        match p1 {
            Some(p1) => Self::from_token((p1, p2)).is_some(),
            None => false,
        }
    }
}
