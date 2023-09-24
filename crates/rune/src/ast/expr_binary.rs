use core::fmt;

use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprBinary>("42 + b");
    rt::<ast::ExprBinary>("b << 10");
}

/// A binary expression.
///
/// * `<expr> <op> <expr>`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprBinary {
    /// Attributes associated with the binary expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The left-hand side of a binary operation.
    pub lhs: Box<ast::Expr>,
    /// The operator.
    pub op: BinOp,
    /// The right-hand side of a binary operation.
    pub rhs: Box<ast::Expr>,
}

expr_parse!(Binary, ExprBinary, "binary expression");

/// A binary operation.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Hash, ToTokens, Spanned)]
#[try_clone(copy)]
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
    /// Type coercion `a as b`.
    As(T![as]),
    /// Instance of test `a is b`.
    Is(T![is]),
    /// Negated instance of test `a is not b`.
    IsNot(T![is not]),
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
            Self::Is(..) | Self::IsNot(..) => 13,
            Self::As(..) => 13,
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
        let ast::Token { kind, span } = p.tok_at(0);

        let out = match kind {
            K![+] => Self::Add(ast::Plus { span }),
            K![-] => Self::Sub(ast::Dash { span }),
            K![*] => Self::Mul(ast::Star { span }),
            K![/] => Self::Div(ast::Div { span }),
            K![%] => Self::Rem(ast::Perc { span }),
            K![==] => Self::Eq(ast::EqEq { span }),
            K![!=] => Self::Neq(ast::BangEq { span }),
            K![<] => Self::Lt(ast::Lt { span }),
            K![>] => Self::Gt(ast::Gt { span }),
            K![<=] => Self::Lte(ast::LtEq { span }),
            K![>=] => Self::Gte(ast::GtEq { span }),
            K![as] => Self::As(ast::As { span }),
            K![is] => {
                let is = ast::Is { span };
                let ast::Token { kind, span } = p.tok_at(1);

                match kind {
                    K![not] => Self::IsNot(ast::IsNot {
                        is,
                        not: ast::Not { span },
                    }),
                    _ => Self::Is(is),
                }
            }
            K![&&] => Self::And(ast::AmpAmp { span }),
            K![||] => Self::Or(ast::PipePipe { span }),
            K![<<] => Self::Shl(ast::LtLt { span }),
            K![>>] => Self::Shr(ast::GtGt { span }),
            K![&] => Self::BitAnd(ast::Amp { span }),
            K![^] => Self::BitXor(ast::Caret { span }),
            K![|] => Self::BitOr(ast::Pipe { span }),
            K![+=] => Self::AddAssign(ast::PlusEq { span }),
            K![-=] => Self::SubAssign(ast::DashEq { span }),
            K![*=] => Self::MulAssign(ast::StarEq { span }),
            K![/=] => Self::DivAssign(ast::SlashEq { span }),
            K![%=] => Self::RemAssign(ast::PercEq { span }),
            K![&=] => Self::BitAndAssign(ast::AmpEq { span }),
            K![^=] => Self::BitXorAssign(ast::CaretEq { span }),
            K![|=] => Self::BitOrAssign(ast::PipeEq { span }),
            K![<<=] => Self::ShlAssign(ast::LtLtEq { span }),
            K![>>=] => Self::ShrAssign(ast::GtGtEq { span }),
            K![..] => Self::DotDot(ast::DotDot { span }),
            K![..=] => Self::DotDotEq(ast::DotDotEq { span }),
            _ => return None,
        };

        Some(out)
    }

    /// Get how many tokens to advance for this operator.
    pub(crate) fn advance(&self, p: &mut Parser<'_>) -> Result<()> {
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
            Self::As(..) => write!(f, "as"),
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
