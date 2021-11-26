use crate::ast::prelude::*;
use std::fmt;

/// A binary expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprBinary {
    /// Attributes associated with the binary expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The left-hand side of a binary operation.
    pub lhs: ast::Expr,
    /// The operator.
    pub op: BinOp,
    /// The right-hand side of a binary operation.
    pub rhs: ast::Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ToTokens, Spanned)]
#[non_exhaustive]
pub struct IsNot {
    /// The `is` token.
    pub is: ast::Is,
    /// The `not` token.
    pub not: ast::Not,
}

expr_parse!(Binary, ExprBinary, "binary expression");

/// A binary operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ToTokens, Spanned)]
#[non_exhaustive]
pub enum BinOp {
    /// Addition `a + b`.
    Add(T![+]),
    /// Subtraction `a - b`.
    Sub(T![-]),
    /// Division `a / b`.
    Div(T![/]),
    /// Multiplication `a * b`.
    Mul(T![*]),
    /// Remainder operator `a % b`.
    Rem(T![%]),
    /// Equality check `a == b`.
    Eq(T![==]),
    /// Inequality check `a != b`.
    Neq(T![!=]),
    /// Greater-than check `a > b`.
    Gt(T![>]),
    /// Less-than check `a < b`.
    Lt(T![<]),
    /// Greater-than or equal check `a >= b`.
    Gte(T![>=]),
    /// Less-than or equal check `a <= b`.
    Lte(T![<=]),
    /// Instance of test `a is b`.
    Is(T![is]),
    /// Negated instance of test `a is not b`.
    IsNot(IsNot),
    /// Lazy and operator `&&`.
    And(T![&&]),
    /// Lazy or operator `||`.
    Or(T![||]),
    /// Bitwise left shift operator `a << b`.
    Shl(T![<<]),
    /// Bitwise right shift operator `a >> b`.
    Shr(T![>>]),
    /// Bitwise and operator `a & b`.
    BitAnd(T![&]),
    /// Bitwise xor operator `a ^ b`.
    BitXor(T![^]),
    /// Bitwise or operator `a | b`.
    BitOr(T![|]),
    /// Add assign `a += b`.
    AddAssign(T![+=]),
    /// Sub assign `a -= b`.
    SubAssign(T![-=]),
    /// Multiply assign operation `a *= b`.
    MulAssign(T![*=]),
    /// Div assign `a /= b`.
    DivAssign(T![/=]),
    /// Remainder assign `a %= b`.
    RemAssign(T![%=]),
    /// Bitwise and assign `a &= b`.
    BitAndAssign(T![&=]),
    /// Bitwise xor assign `a ^= b`.
    BitXorAssign(T![^=]),
    /// Bitwise or assign `a |= b`.
    BitOrAssign(T![|=]),
    /// Left shift assign `a <<= b`.
    ShlAssign(T![<<=]),
    /// Right shift assign `a >>= b`.
    ShrAssign(T![>>=]),
    /// `a .. b`.
    DotDot(T![..]),
    /// `a ..= b`.
    DotDotEq(T![..=]),
}

impl BinOp {
    /// Test if operator is an assign operator.
    pub(crate) fn is_assign(&self) -> bool {
        match self {
            Self::AddAssign(..) => true,
            Self::SubAssign(..) => true,
            Self::MulAssign(..) => true,
            Self::DivAssign(..) => true,
            Self::RemAssign(..) => true,
            Self::BitAndAssign(..) => true,
            Self::BitXorAssign(..) => true,
            Self::BitOrAssign(..) => true,
            Self::ShlAssign(..) => true,
            Self::ShrAssign(..) => true,
            _ => false,
        }
    }

    /// Test if operator is a condiational operator.
    pub(crate) fn is_conditional(self) -> bool {
        match self {
            Self::And(..) => true,
            Self::Or(..) => true,
            _ => false,
        }
    }

    /// Get the precedence for the current operator.
    pub(super) fn precedence(&self) -> usize {
        // NB: Rules from: https://doc.rust-lang.org/reference/expressions.html#expression-precedence
        match self {
            Self::Is(..) | Self::IsNot(..) => 12,
            Self::Mul(..) | Self::Div(..) | Self::Rem(..) => 11,
            Self::Add(..) | Self::Sub(..) => 10,
            Self::Shl(..) | Self::Shr(..) => 9,
            Self::BitAnd(..) => 8,
            Self::BitXor(..) => 7,
            Self::BitOr(..) => 6,
            Self::Eq(..)
            | Self::Neq(..)
            | Self::Lt(..)
            | Self::Gt(..)
            | Self::Lte(..)
            | Self::Gte(..) => 5,
            Self::And(..) => 4,
            Self::Or(..) => 3,
            Self::DotDot(..) | Self::DotDotEq(..) => 2,
            // assign operators
            _ => 1,
        }
    }

    /// Test if operator is left associative.
    pub(super) fn is_assoc(&self) -> bool {
        match self {
            Self::Mul(..) => true,
            Self::Div(..) => true,
            Self::Add(..) => true,
            Self::Sub(..) => true,
            Self::Or(..) => true,
            Self::And(..) => true,
            Self::Rem(..) => true,
            Self::Shl(..) => true,
            Self::Shr(..) => true,
            Self::BitAnd(..) => true,
            Self::BitOr(..) => true,
            Self::BitXor(..) => true,
            _ => false,
        }
    }

    /// Convert from a token.
    pub(super) fn from_peeker(p: &mut Peeker<'_>) -> Option<BinOp> {
        let token = p.tok_at(0);

        let out = match token.kind {
            K![+] => Self::Add(ast::Plus { token }),
            K![-] => Self::Sub(ast::Dash { token }),
            K![*] => Self::Mul(ast::Star { token }),
            K![/] => Self::Div(ast::Div { token }),
            K![%] => Self::Rem(ast::Perc { token }),
            K![==] => Self::Eq(ast::EqEq { token }),
            K![!=] => Self::Neq(ast::BangEq { token }),
            K![<] => Self::Lt(ast::Lt { token }),
            K![>] => Self::Gt(ast::Gt { token }),
            K![<=] => Self::Lte(ast::LtEq { token }),
            K![>=] => Self::Gte(ast::GtEq { token }),
            K![is] => {
                let is = ast::Is { token };
                let token = p.tok_at(1);

                match token.kind {
                    K![not] => Self::IsNot(IsNot {
                        is,
                        not: ast::Not { token },
                    }),
                    _ => Self::Is(is),
                }
            }
            K![&&] => Self::And(ast::AmpAmp { token }),
            K![||] => Self::Or(ast::PipePipe { token }),
            K![<<] => Self::Shl(ast::LtLt { token }),
            K![>>] => Self::Shr(ast::GtGt { token }),
            K![&] => Self::BitAnd(ast::Amp { token }),
            K![^] => Self::BitXor(ast::Caret { token }),
            K![|] => Self::BitOr(ast::Pipe { token }),
            K![+=] => Self::AddAssign(ast::PlusEq { token }),
            K![-=] => Self::SubAssign(ast::DashEq { token }),
            K![*=] => Self::MulAssign(ast::StarEq { token }),
            K![/=] => Self::DivAssign(ast::SlashEq { token }),
            K![%=] => Self::RemAssign(ast::PercEq { token }),
            K![&=] => Self::BitAndAssign(ast::AmpEq { token }),
            K![^=] => Self::BitXorAssign(ast::CaretEq { token }),
            K![|=] => Self::BitOrAssign(ast::PipeEq { token }),
            K![<<=] => Self::ShlAssign(ast::LtLtEq { token }),
            K![>>=] => Self::ShrAssign(ast::GtGtEq { token }),
            K![..] => Self::DotDot(ast::DotDot { token }),
            K![..=] => Self::DotDotEq(ast::DotDotEq { token }),
            _ => return None,
        };

        Some(out)
    }

    /// Get how many tokens to advance for this operator.
    pub(crate) fn advance(&self, p: &mut Parser<'_>) -> Result<(), ParseError> {
        match self {
            Self::IsNot(..) => {
                p.next()?;
                p.next()?;
            }
            _ => {
                p.next()?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Add(..) => write!(f, "+"),
            Self::Sub(..) => write!(f, "-"),
            Self::Div(..) => write!(f, "/"),
            Self::Mul(..) => write!(f, "*"),
            Self::Rem(..) => write!(f, "%"),
            Self::Eq(..) => write!(f, "=="),
            Self::Neq(..) => write!(f, "!="),
            Self::Gt(..) => write!(f, ">"),
            Self::Lt(..) => write!(f, "<"),
            Self::Gte(..) => write!(f, ">="),
            Self::Lte(..) => write!(f, "<="),
            Self::Is(..) => write!(f, "is"),
            Self::IsNot(..) => write!(f, "is not"),
            Self::And(..) => write!(f, "&&"),
            Self::Or(..) => write!(f, "||"),
            Self::Shl(..) => write!(f, "<<"),
            Self::Shr(..) => write!(f, ">>"),
            Self::BitAnd(..) => write!(f, "&"),
            Self::BitXor(..) => write!(f, "^"),
            Self::BitOr(..) => write!(f, "|"),
            Self::AddAssign(..) => write!(f, "+="),
            Self::SubAssign(..) => write!(f, "-="),
            Self::DivAssign(..) => write!(f, "/="),
            Self::MulAssign(..) => write!(f, "*="),
            Self::BitAndAssign(..) => write!(f, "&="),
            Self::BitXorAssign(..) => write!(f, "^="),
            Self::BitOrAssign(..) => write!(f, "|="),
            Self::RemAssign(..) => write!(f, "%="),
            Self::ShlAssign(..) => write!(f, "<<="),
            Self::ShrAssign(..) => write!(f, ">>="),
            Self::DotDot(..) => write!(f, ".."),
            Self::DotDotEq(..) => write!(f, "..="),
        }
    }
}

impl Peek for BinOp {
    fn peek(p: &mut Peeker<'_>) -> bool {
        Self::from_peeker(p).is_some()
    }
}
