use crate::ast;
use crate::ast::utils;
use crate::{IntoTokens, Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// A select expression that selects over a collection of futures.
#[derive(Debug, Clone)]
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

impl Spanned for ExprSelect {
    fn span(&self) -> Span {
        self.select.span().join(self.close.span())
    }
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

                let comma = if parser.peek::<ast::Comma>()? {
                    Some(parser.parse()?)
                } else {
                    None
                };

                is_end = utils::is_block_end(&*branch.body, comma.as_ref());
                default_branch = Some((branch, comma));
            } else {
                let branch = parser.parse::<ExprSelectBranch>()?;

                let comma = if parser.peek::<ast::Comma>()? {
                    Some(parser.parse()?)
                } else {
                    None
                };

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

impl IntoTokens for ExprSelect {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.select.into_tokens(context, stream);
        self.open.into_tokens(context, stream);
        self.branches.into_tokens(context, stream);
        self.default_branch.into_tokens(context, stream);
        self.close.into_tokens(context, stream);
    }
}

/// A single selection branch.
#[derive(Debug, Clone)]
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

impl IntoTokens for ExprSelectBranch {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.pat.into_tokens(context, stream);
        self.eq.into_tokens(context, stream);
        self.expr.into_tokens(context, stream);
        self.rocket.into_tokens(context, stream);
        self.body.into_tokens(context, stream);
    }
}

/// A single selection branch.
#[derive(Debug, Clone)]
pub struct ExprDefaultBranch {
    /// The `default` keyword.
    pub default: ast::Default,
    /// `=>`.
    pub rocket: ast::Rocket,
    /// The body of the expression.
    pub body: Box<ast::Expr>,
}

impl ExprDefaultBranch {
    /// The span of the expression.
    pub fn span(&self) -> Span {
        self.default.span().join(self.body.span())
    }
}

impl Parse for ExprDefaultBranch {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            default: parser.parse()?,
            rocket: parser.parse()?,
            body: Box::new(parser.parse()?),
        })
    }
}

impl IntoTokens for ExprDefaultBranch {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.default.into_tokens(context, stream);
        self.rocket.into_tokens(context, stream);
        self.body.into_tokens(context, stream);
    }
}
