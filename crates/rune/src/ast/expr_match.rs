use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// A match expression.
#[derive(Debug, Clone)]
pub struct ExprMatch {
    /// The `match` token.
    pub match_: ast::Match,
    /// The expression who's result we match over.
    pub expr: Box<ast::Expr>,
    /// The open brace of the match.
    pub open: ast::OpenBrace,
    /// Branches.
    pub branches: Vec<(ExprMatchBranch, Option<ast::Comma>)>,
    /// The close brace of the match.
    pub close: ast::CloseBrace,
}

into_tokens!(ExprMatch {
    match_,
    expr,
    open,
    branches,
    close
});

impl Spanned for ExprMatch {
    fn span(&self) -> Span {
        self.match_.span().join(self.close.span())
    }
}

/// Parse a match statement.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprMatch>("match 0 { _ => 1, }").unwrap();
/// ```
impl Parse for ExprMatch {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let match_ = parser.parse()?;
        let expr = Box::new(ast::Expr::parse_without_eager_brace(parser)?);

        let open = parser.parse()?;

        let mut branches = Vec::new();

        while !parser.peek::<ast::CloseBrace>()? {
            let branch = parser.parse::<ExprMatchBranch>()?;

            let comma = if parser.peek::<ast::Comma>()? {
                Some(parser.parse()?)
            } else {
                None
            };

            let is_end = ast::utils::is_block_end(&*branch.body, comma.as_ref());
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

/// A match branch.
#[derive(Debug, Clone)]
pub struct ExprMatchBranch {
    /// The pattern to match.
    pub pat: ast::Pat,
    /// The branch condition.
    pub condition: Option<(ast::If, Box<ast::Expr>)>,
    /// The rocket token.
    pub rocket: ast::Rocket,
    /// The body of the match.
    pub body: Box<ast::Expr>,
}

into_tokens!(ExprMatchBranch {
    pat,
    condition,
    rocket,
    body
});

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
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprMatchBranch>("1 => { foo }").unwrap();
/// ```
impl Parse for ExprMatchBranch {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let pat = parser.parse()?;

        let condition = if parser.peek::<ast::If>()? {
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
