use crate::ast;
use crate::ast::utils;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A `select` expression that selects over a collection of futures.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// let select = testing::roundtrip::<ast::ExprSelect>(r#"
/// select {
///     _ = a => 0,
///     _ = b => {},
///     _ = c => {}
///     default => ()
/// }
/// "#);
///
/// assert_eq!(4, select.branches.len());
/// assert!(matches!(select.branches.get(1), Some(&(ast::ExprSelectBranch::Pat(..), Some(..)))));
/// assert!(matches!(select.branches.get(2), Some(&(ast::ExprSelectBranch::Pat(..), None))));
/// assert!(matches!(select.branches.get(3), Some(&(ast::ExprSelectBranch::Default(..), None))));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprSelect {
    /// The attributes of the `select`
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The `select` keyword.
    pub select: ast::Select,
    /// The open brace.
    pub open: ast::OpenBrace,
    /// The branches of the select.
    pub branches: Vec<(ExprSelectBranch, Option<ast::Comma>)>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl ExprSelect {
    /// Parse the `select` expression and attach the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let select = parser.parse()?;
        let open = parser.parse()?;

        let mut branches = Vec::new();

        while !parser.peek::<ast::CloseBrace>()? {
            let branch = ExprSelectBranch::parse(parser)?;
            let comma = parser.parse::<Option<ast::Comma>>()?;
            let is_end = utils::is_block_end(branch.expr(), comma.as_ref());
            branches.push((branch, comma));

            if is_end {
                break;
            }
        }

        let close = parser.parse()?;

        Ok(Self {
            attributes,
            select,
            open,
            branches,
            close,
        })
    }
}

expr_parse!(ExprSelect, "select expression");

/// A single selection branch.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum ExprSelectBranch {
    /// A patterned branch.
    Pat(ExprSelectPatBranch),
    /// A default branch.
    Default(ExprDefaultBranch),
}

impl ExprSelectBranch {
    /// Access the expression body.
    pub fn expr(&self) -> &ast::Expr {
        match self {
            ExprSelectBranch::Pat(pat) => &pat.body,
            ExprSelectBranch::Default(def) => &def.body,
        }
    }
}

impl Parse for ExprSelectBranch {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(if parser.peek::<ast::Default>()? {
            Self::Default(parser.parse()?)
        } else {
            Self::Pat(parser.parse()?)
        })
    }
}

/// A single selection branch.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprSelectPatBranch {
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
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprDefaultBranch {
    /// The `default` keyword.
    pub default: ast::Default,
    /// `=>`.
    pub rocket: ast::Rocket,
    /// The body of the expression.
    pub body: Box<ast::Expr>,
}
