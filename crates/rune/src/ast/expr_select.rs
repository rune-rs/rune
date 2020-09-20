use crate::ast;
use crate::ast::utils;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A select expression that selects over a collection of futures.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprSelect {
    /// The `select` keyword.
    pub select: ast::Select,
    /// The opening brace of the select.
    pub open: ast::OpenBrace,
    /// The branches of the select.
    pub branches: Vec<(ExprSelectBranch, Option<ast::Comma>)>,
    /// The default branch.
    pub default_branch: Option<(ExprDefaultBranch, Option<ast::Comma>)>,
    /// The closing brace of the select.
    pub close: ast::CloseBrace,
}

impl Parse for ExprSelect {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let select = parser.parse()?;
        let open = parser.parse()?;

        let mut branches = Vec::new();
        let mut default_branch = None;

        while !parser.peek::<ast::CloseBrace>()? {
            let is_end;

            if parser.peek::<ast::Default>()? {
                let branch = parser.parse::<ExprDefaultBranch>()?;
                let comma = parser.parse::<Option<ast::Comma>>()?;

                is_end = utils::is_block_end(&*branch.body, comma.as_ref());
                default_branch = Some((branch, comma));
            } else {
                let branch = parser.parse::<ExprSelectBranch>()?;
                let comma = parser.parse::<Option<ast::Comma>>()?;

                is_end = utils::is_block_end(&*branch.body, comma.as_ref());
                branches.push((branch, comma));
            };

            if is_end {
                break;
            }
        }

        let close = parser.parse()?;

        Ok(Self {
            select,
            open,
            branches,
            default_branch,
            close,
        })
    }
}

/// A single selection branch.
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
pub struct ExprSelectBranch {
    /// The identifier to bind the result to.
    pub pat: ast::Pat,
    /// `=`.
    pub eq: ast::Eq,
    /// The expression that should evaluate to a future.
    pub expr: Box<ast::Expr>,
    /// `=>`.
    pub rocket: ast::Rocket,
    /// The body of the expression.
    pub body: Box<ast::Expr>,
}

/// A single selection branch.
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
pub struct ExprDefaultBranch {
    /// The `default` keyword.
    pub default: ast::Default,
    /// `=>`.
    pub rocket: ast::Rocket,
    /// The body of the expression.
    pub body: Box<ast::Expr>,
}
