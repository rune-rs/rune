use crate::ast::utils;
use crate::ast::{CloseBrace, Comma, Expr, If, Match, OpenBrace, Pat, Rocket};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

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
}

impl ExprMatch {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.match_.span().join(self.close.span())
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
        let expr = Box::new(Expr::parse_without_eager_brace(parser)?);

        let open = parser.parse()?;

        let mut branches = Vec::new();

        while !parser.peek::<CloseBrace>()? {
            let branch = parser.parse::<ExprMatchBranch>()?;

            let comma = if parser.peek::<Comma>()? {
                Some(parser.parse()?)
            } else {
                None
            };

            let is_end = utils::is_block_end(&*branch.body, comma.as_ref());
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
        })
    }
}
