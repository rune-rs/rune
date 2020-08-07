use crate::ast::utils;
use crate::ast::{CloseBrace, Comma, Eq, Expr, OpenBrace, Pat, Rocket, Select};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use stk::unit::Span;

/// A single selection branch.
#[derive(Debug, Clone)]
pub struct ExprSelectBranch {
    /// The identifier to bind the result to.
    pub pat: Pat,
    /// `=`.
    pub eq: Eq,
    /// The expression that should evaluate to a future.
    pub expr: Box<Expr>,
    /// `=>`.
    pub rocket: Rocket,
    /// The body of the expression.
    pub body: Box<Expr>,
}

impl ExprSelectBranch {
    /// The span of the expression.
    pub fn span(&self) -> Span {
        self.pat.span().join(self.body.span())
    }
}

impl Parse for ExprSelectBranch {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            pat: parser.parse()?,
            eq: parser.parse()?,
            expr: Box::new(parser.parse()?),
            rocket: parser.parse()?,
            body: Box::new(parser.parse()?),
        })
    }
}

/// A select expression that selects over a collection of futures.
#[derive(Debug, Clone)]
pub struct ExprSelect {
    /// The `select` keyword.
    pub select: Select,
    /// The opening brace of the select.
    pub open: OpenBrace,
    /// The branches of the select.
    pub branches: Vec<(ExprSelectBranch, Option<Comma>)>,
    /// The closing brace of the select.
    pub close: CloseBrace,
}

impl ExprSelect {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.select.span().join(self.close.span())
    }
}

impl Parse for ExprSelect {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let select = parser.parse()?;
        let open = parser.parse()?;

        let mut branches = Vec::new();

        while !parser.peek::<CloseBrace>()? {
            let branch = parser.parse::<ExprSelectBranch>()?;

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

        Ok(Self {
            select,
            open,
            branches,
            close,
        })
    }
}
