use crate::ast;
use crate::{ParseError, Parser, Peek, Peeker, Spanned, ToTokens};
use runestick::Span;
use std::fmt;

/// A binary expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprBinary {
    /// Attributes associated with the binary expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The left-hand side of a binary operation.
    pub lhs: ast::Expr,
    /// Token associated with operator.
    pub t1: ast::Token,
    /// Token associated with optional second part of operator.
    pub t2: Option<ast::Token>,
    /// The right-hand side of a binary operation.
    pub rhs: ast::Expr,
    /// The operation to apply.
    #[rune(skip)]
    pub op: BinOp,
}

impl ExprBinary {
    /// Get the span of the op.
    pub fn op_span(&self) -> Span {
        if let Some(t2) = self.t2 {
            self.t1.span().join(t2.span())
        } else {
            self.t1.span()
        }
    }
}

expr_parse!(Binary, ExprBinary, "binary expression");

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
    /// `a ..= b`.
    DotDot,
    /// `a ..= b`.
    DotDotEq,
}

impl BinOp {
    /// Test if operator is an assign operator.
    pub fn is_assign(self) -> bool {
        matches!(
            self,
            Self::AddAssign
                | Self::SubAssign
                | Self::MulAssign
                | Self::DivAssign
                | Self::RemAssign
                | Self::BitAndAssign
                | Self::BitXorAssign
                | Self::BitOrAssign
                | Self::ShlAssign
                | Self::ShrAssign
        )
    }

    /// Test if operator is a condiational operator.
    pub fn is_conditional(self) -> bool {
        matches!(self, Self::And | Self::Or)
    }

    /// Get the precedence for the current operator.
    pub(super) fn precedence(self) -> usize {
        // NB: Rules from: https://doc.rust-lang.org/reference/expressions.html#expression-precedence
        match self {
            Self::Is | Self::IsNot => 12,
            Self::Mul | Self::Div | Self::Rem => 11,
            Self::Add | Self::Sub => 10,
            Self::Shl | Self::Shr => 9,
            Self::BitAnd => 8,
            Self::BitXor => 7,
            Self::BitOr => 6,
            Self::Eq | Self::Neq | Self::Lt | Self::Gt | Self::Lte | Self::Gte => 5,
            Self::And => 4,
            Self::Or => 3,
            Self::DotDot | Self::DotDotEq => 2,
            // assign operators
            _ => 1,
        }
    }

    /// Test if operator is left associative.
    pub(super) fn is_assoc(self) -> bool {
        matches!(
            self,
            Self::Mul
                | Self::Div
                | Self::Add
                | Self::Sub
                | Self::Or
                | Self::And
                | Self::Rem
                | Self::Shl
                | Self::Shr
                | Self::BitAnd
                | Self::BitOr
                | Self::BitXor
        )
    }

    /// Convert from a token.
    pub(super) fn from_peeker(p: &mut Peeker<'_>) -> Option<BinOp> {
        Some(match p.nth(0) {
            K![+] => Self::Add,
            K![-] => Self::Sub,
            K![*] => Self::Mul,
            K![/] => Self::Div,
            K![%] => Self::Rem,
            K![==] => Self::Eq,
            K![!=] => Self::Neq,
            K![<] => Self::Lt,
            K![>] => Self::Gt,
            K![<=] => Self::Lte,
            K![>=] => Self::Gte,
            ast::Kind::Is => match p.nth(1) {
                K![not] => Self::IsNot,
                _ => Self::Is,
            },
            K![&&] => Self::And,
            K![||] => Self::Or,
            K![<<] => Self::Shl,
            K![>>] => Self::Shr,
            K![&] => Self::BitAnd,
            K![^] => Self::BitXor,
            K![|] => Self::BitOr,
            K![+=] => Self::AddAssign,
            K![-=] => Self::SubAssign,
            K![*=] => Self::MulAssign,
            K![/=] => Self::DivAssign,
            K![%=] => Self::RemAssign,
            K![&=] => Self::BitAndAssign,
            K![^=] => Self::BitXorAssign,
            K![|=] => Self::BitOrAssign,
            K![<<=] => Self::ShlAssign,
            K![>>=] => Self::ShrAssign,
            K![..] => Self::DotDot,
            K![..=] => Self::DotDotEq,
            _ => return None,
        })
    }

    /// Get how many tokens to advance for this operator.
    pub(crate) fn advance(
        &self,
        p: &mut Parser<'_>,
    ) -> Result<(ast::Token, Option<ast::Token>), ParseError> {
        Ok(match self {
            Self::IsNot => (p.next()?, Some(p.next()?)),
            _ => (p.next()?, None),
        })
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
            Self::DotDot => write!(f, ".."),
            Self::DotDotEq => write!(f, "..="),
        }
    }
}

impl Peek for BinOp {
    fn peek(p: &mut Peeker<'_>) -> bool {
        Self::from_peeker(p).is_some()
    }
}
