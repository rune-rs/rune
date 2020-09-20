use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A match expression.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprMatch {
    /// The attributes for the match expression
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
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

impl ExprMatch {
    /// Parse the `match` expression attaching the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
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
            attributes,
            match_,
            expr,
            open,
            branches,
            close,
        })
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
/// let expr = parse_all::<ast::ExprMatch>("#[jit(always)] match 0 { _ => 1, }").unwrap();
/// assert_eq!(expr.attributes.len(), 1);
/// ```
impl Parse for ExprMatch {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}

/// A match branch.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprMatchBranch>("1 => { foo }").unwrap();
/// ```
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
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

impl ExprMatchBranch {
    /// Test if the branch produces nothing.
    pub fn produces_nothing(&self) -> bool {
        self.body.produces_nothing()
    }
}
