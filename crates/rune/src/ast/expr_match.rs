use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A match expression.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprMatch>("match 0 { _ => 1, }");
/// let expr = testing::roundtrip::<ast::ExprMatch>("#[jit(always)] match 0 { _ => 1, }");
/// assert_eq!(expr.attributes.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprMatch {
    /// The attributes for the match expression
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The `match` token.
    pub match_: T![match],
    /// The expression who's result we match over.
    pub expr: ast::Expr,
    /// The open brace of the match.
    pub open: T!['{'],
    /// Branches.
    pub branches: Vec<(ExprMatchBranch, Option<T![,]>)>,
    /// The close brace of the match.
    pub close: T!['}'],
}

impl ExprMatch {
    /// Parse the `match` expression attaching the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let match_ = parser.parse()?;
        let expr = ast::Expr::parse_without_eager_brace(parser)?;

        let open = parser.parse()?;

        let mut branches = Vec::new();

        while !parser.peek::<ast::CloseBrace>()? {
            let branch = parser.parse::<ExprMatchBranch>()?;
            let comma = parser.parse::<Option<T![,]>>()?;
            let is_end = ast::utils::is_block_end(&branch.body, comma.as_ref());
            branches.push((branch, comma));

            if is_end {
                break;
            }
        }

        let close = parser.parse()?;

        Ok(ExprMatch {
            attributes,
            match_,
            expr,
            open,
            branches,
            close,
        })
    }
}

expr_parse!(ExprMatch, "match expression");

/// A match branch.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprMatchBranch>("1 => { foo }");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprMatchBranch {
    /// The pattern to match.
    pub pat: ast::Pat,
    /// The branch condition.
    pub condition: Option<(T![if], ast::Expr)>,
    /// The rocket token.
    pub rocket: T![=>],
    /// The body of the match.
    pub body: ast::Expr,
}
