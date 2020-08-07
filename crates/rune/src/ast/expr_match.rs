use crate::ast::{CloseBrace, Comma, Expr, If, Match, OpenBrace, Pat, Rocket};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use stk::unit::Span;

/// A match branch.
#[derive(Debug, Clone)]
pub struct ExprMatchBranch {
    /// The pattern to match.
    pub pat: Pat,
    /// The branch condition.
    pub condition: Option<(If, Box<Expr>)>,
    /// The rocket token.
    pub rocket: Rocket,
    /// The body of the match.
    pub body: Box<Expr>,
}

impl ExprMatchBranch {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.pat.span().join(self.body.span())
    }

    /// Test if the branch produces nothing.
    pub fn produces_nothing(&self) -> bool {
        self.body.produces_nothing()
    }
}

/// Parse a match statement.
///
/// # Examples
///
/// ```rust
/// use rune::{ParseAll, parse_all, ast};
///
/// # fn main() -> rune::Result<()> {
/// parse_all::<ast::ExprMatchBranch>("1 => { foo }")?;
/// # Ok(())
/// # }
/// ```
impl Parse for ExprMatchBranch {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let pat = parser.parse()?;

        let condition = if parser.peek::<If>()? {
            Some((parser.parse()?, Box::new(parser.parse()?)))
        } else {
            None
        };

        Ok(Self {
            pat,
            condition,
            rocket: parser.parse()?,
            body: Box::new(parser.parse()?),
        })
    }
}

/// A match expression.
#[derive(Debug, Clone)]
pub struct ExprMatch {
    /// The `match` token.
    pub match_: Match,
    /// The expression who's result we match over.
    pub expr: Box<Expr>,
    /// The open brace of the match.
    pub open: OpenBrace,
    /// Branches.
    pub branches: Vec<(ExprMatchBranch, Option<Comma>)>,
    /// The close brace of the match.
    pub close: CloseBrace,
    /// Test if expression has a default branch.
    pub has_default: bool,
}

impl ExprMatch {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.match_.span().join(self.close.span())
    }

    /// An if statement evaluates to empty if it does not have an else branch.
    pub fn produces_nothing(&self) -> bool {
        !self.has_default
    }
}

/// Parse a match statement.
///
/// # Examples
///
/// ```rust
/// use rune::{ParseAll, parse_all, ast};
///
/// # fn main() -> rune::Result<()> {
/// parse_all::<ast::ExprMatch>("match 0 { _ => 1, }")?;
/// # Ok(())
/// # }
/// ```
impl Parse for ExprMatch {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let match_ = parser.parse()?;
        let expr = Box::new(parser.parse()?);

        let open = parser.parse()?;

        let mut branches = Vec::new();
        let mut default_branch = None::<Span>;

        while !parser.peek::<CloseBrace>()? {
            let branch = parser.parse::<ExprMatchBranch>()?;

            let comma = if parser.peek::<Comma>()? {
                Some(parser.parse()?)
            } else {
                None
            };

            let fallback = match &branch.pat {
                Pat::PatIgnore(..) | Pat::PatBinding(..) if branch.condition.is_none() => {
                    Some(branch.span())
                }
                _ => None,
            };

            default_branch = match (fallback, default_branch) {
                (Some(span), Some(existing)) => {
                    return Err(ParseError::MatchMultipleFallbackBranches { span, existing });
                }
                (None, Some(existing)) => {
                    return Err(ParseError::MatchNeverReached {
                        span: branch.span(),
                        existing,
                    });
                }
                (Some(fallback), None) => Some(fallback),
                (_, default_branch) => default_branch,
            };

            let is_end = match (&*branch.body, &comma) {
                (Expr::ExprBlock(..), _) => false,
                (Expr::ExprFor(..), _) => false,
                (Expr::ExprWhile(..), _) => false,
                (Expr::ExprIf(..), _) => false,
                (Expr::ExprMatch(..), _) => false,
                (_, Some(..)) => false,
                (_, None) => true,
            };

            branches.push((branch, comma));

            if is_end {
                break;
            }
        }

        let close = parser.parse()?;

        Ok(ExprMatch {
            match_,
            expr,
            open,
            branches,
            close,
            has_default: default_branch.is_some(),
        })
    }
}
