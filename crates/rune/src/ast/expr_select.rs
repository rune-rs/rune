use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    let select = rt::<ast::ExprSelect>(
        r#"
    select {
        _ = a => 0,
        _ = b => {},
        _ = c => {}
        default => ()
    }
    "#,
    );

    assert_eq!(4, select.branches.len());
    assert!(matches!(
        select.branches.get(1),
        Some(&(ast::ExprSelectBranch::Pat(..), Some(..)))
    ));
    assert!(matches!(
        select.branches.get(2),
        Some(&(ast::ExprSelectBranch::Pat(..), None))
    ));
    assert!(matches!(
        select.branches.get(3),
        Some(&(ast::ExprSelectBranch::Default(..), None))
    ));
}

/// A `select` expression that selects over a collection of futures.
///
/// * `select { [arm]* }`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprSelect {
    /// The attributes of the `select`
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The `select` keyword.
    pub select: T![select],
    /// The open brace.
    pub open: T!['{'],
    /// The branches of the select.
    #[rune(iter)]
    pub branches: Vec<(ExprSelectBranch, Option<T![,]>)>,
    /// The close brace.
    pub close: T!['}'],
}

impl ExprSelect {
    /// Parse the `select` expression and attach the given attributes
    pub(crate) fn parse_with_attributes(
        p: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self> {
        let select = p.parse()?;
        let open = p.parse()?;

        let mut branches = Vec::new();

        while !p.peek::<T!['}']>()? {
            let branch = ExprSelectBranch::parse(p)?;
            let comma = p.parse::<Option<T![,]>>()?;
            let is_end = ast::utils::is_block_end(branch.expr(), comma.as_ref());
            branches.try_push((branch, comma))?;

            if is_end {
                break;
            }
        }

        let close = p.parse()?;

        Ok(Self {
            attributes,
            select,
            open,
            branches,
            close,
        })
    }
}

expr_parse!(Select, ExprSelect, "select expression");

/// A single selection branch.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum ExprSelectBranch {
    /// A patterned branch.
    Pat(ExprSelectPatBranch),
    /// A default branch.
    Default(ExprDefaultBranch),
}

impl ExprSelectBranch {
    /// Access the expression body.
    pub(crate) fn expr(&self) -> &ast::Expr {
        match self {
            ExprSelectBranch::Pat(pat) => &pat.body,
            ExprSelectBranch::Default(def) => &def.body,
        }
    }
}

impl Parse for ExprSelectBranch {
    fn parse(p: &mut Parser) -> Result<Self> {
        Ok(if p.peek::<T![default]>()? {
            Self::Default(p.parse()?)
        } else {
            Self::Pat(p.parse()?)
        })
    }
}

/// A single selection branch.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Parse, Spanned)]
#[non_exhaustive]
pub struct ExprSelectPatBranch {
    /// The identifier to bind the result to.
    pub pat: ast::Pat,
    /// `=`.
    pub eq: T![=],
    /// The expression that should evaluate to a future.
    pub expr: ast::Expr,
    /// `=>`.
    pub rocket: T![=>],
    /// The body of the expression.
    pub body: ast::Expr,
}

/// A single selection branch.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Parse, Spanned)]
#[non_exhaustive]
pub struct ExprDefaultBranch {
    /// The `default` keyword.
    pub default: T![default],
    /// `=>`.
    pub rocket: T![=>],
    /// The body of the expression.
    pub body: ast::Expr,
}
