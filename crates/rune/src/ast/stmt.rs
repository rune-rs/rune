use core::mem::take;

use crate::ast::prelude::*;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
    rt::<ast::Stmt>("let x = 1;");
    rt::<ast::Stmt>("#[attr] let a = f();");
    rt::<ast::Stmt>("line!().bar()");

    // A block-like expression in statement position followed by parenthesis is
    // treated as two statements rather than a call.
    let block = rt::<ast::Block>("{ if true { } (1, 2, 3); }");
    assert_eq!(block.statements.len(), 2);
    assert!(matches!(
        block.statements[0],
        ast::Stmt::Expr(ast::Expr::If(..))
    ));

    let block = rt::<ast::Block>("{ match x { _ => () } (1, 2, 3); }");
    assert_eq!(block.statements.len(), 2);
    assert!(matches!(
        block.statements[0],
        ast::Stmt::Expr(ast::Expr::Match(..))
    ));

    let block = rt::<ast::Block>("{ { } (1, 2, 3); }");
    assert_eq!(block.statements.len(), 2);
    assert!(matches!(
        block.statements[0],
        ast::Stmt::Expr(ast::Expr::Block(..))
    ));

    let block = rt::<ast::Block>("{ if true { } [1, 2, 3]; }");
    assert_eq!(block.statements.len(), 2);

    // In expression position the same construct is a call.
    let expr = rt::<ast::Expr>("if true { a } else { b }(1, 2, 3)");
    assert!(matches!(expr, ast::Expr::Call(..)));
}

/// A statement within a block.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum Stmt {
    /// A local declaration.
    Local(Box<ast::Local>),
    /// A declaration.
    Item(ast::Item, #[rune(iter)] Option<T![;]>),
    /// An expression.
    Expr(ast::Expr),
    /// An with a trailing semi-colon.
    ///
    /// And absent semicolon indicates that it is synthetic.
    Semi(StmtSemi),
}

impl Peek for Stmt {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K![let]) || ItemOrExpr::peek(p)
    }
}

impl Parse for Stmt {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        let mut attributes = p.parse()?;
        let visibility = p.parse()?;

        if ast::Item::peek_as_item(p.peeker()) {
            let path = p.parse::<Option<ast::Path>>()?;
            let item: ast::Item = ast::Item::parse_with_meta_path(p, attributes, visibility, path)?;

            let semi = if item.needs_semi_colon() {
                Some(p.parse()?)
            } else {
                p.parse()?
            };

            return Ok(Self::Item(item, semi));
        }

        if let Some(span) = visibility.option_span() {
            return Err(compile::Error::unsupported(span, "visibility modifier"));
        }

        let stmt = if let K![let] = p.nth(0)? {
            let local = Box::try_new(ast::Local::parse_with_meta(p, take(&mut attributes))?)?;
            Self::Local(local)
        } else {
            let expr = ast::Expr::parse_with_meta(p, &mut attributes, ast::expr::NOT_CALLABLE)?;

            // Parsed an expression which can be treated directly as an item.
            match p.parse()? {
                Some(semi) => Self::Semi(StmtSemi::new(expr, semi)),
                None => Self::Expr(expr),
            }
        };

        if let Some(span) = attributes.option_span() {
            return Err(compile::Error::unsupported(span, "attributes"));
        }

        Ok(stmt)
    }
}

/// Parsing an item or an expression.
#[derive(Debug, TryClone, PartialEq, Eq)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum ItemOrExpr {
    /// An item.
    Item(ast::Item),
    /// An expression.
    Expr(ast::Expr),
}

impl Peek for ItemOrExpr {
    fn peek(p: &mut Peeker<'_>) -> bool {
        match p.nth(0) {
            K![use] => true,
            K![enum] => true,
            K![struct] => true,
            K![impl] => true,
            K![async] => matches!(p.nth(1), K![fn]),
            K![fn] => true,
            K![mod] => true,
            K![const] => true,
            K![ident(..)] => true,
            K![::] => true,
            _ => ast::Expr::peek(p),
        }
    }
}

impl Parse for ItemOrExpr {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        let mut attributes = p.parse()?;
        let visibility = p.parse()?;

        if ast::Item::peek_as_item(p.peeker()) {
            let path = p.parse()?;
            let item: ast::Item = ast::Item::parse_with_meta_path(p, attributes, visibility, path)?;
            return Ok(Self::Item(item));
        }

        if let Some(span) = visibility.option_span() {
            return Err(compile::Error::unsupported(span, "visibility modifier"));
        }

        let expr = ast::Expr::parse_with_meta(p, &mut attributes, ast::expr::NOT_CALLABLE)?;

        if let Some(span) = attributes.option_span() {
            return Err(compile::Error::unsupported(span, "attributes"));
        }

        Ok(Self::Expr(expr))
    }
}

/// Key used to stort a statement into its processing order.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum StmtSortKey {
    /// USe statements, that should be processed first.
    Use,
    /// Items.
    Item,
    /// Other things, that should be processed last.
    Other,
}

/// A semi-terminated expression.
///
/// These have special meaning since they indicate that whatever block or
/// function they belong to should not evaluate to the value of the expression
/// if it is the last expression in the block.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct StmtSemi {
    /// The expression that is considered to be semi-terminated.
    pub expr: ast::Expr,
    /// The semi-token associated with the expression.
    pub semi_token: T![;],
}

impl StmtSemi {
    /// Construct a new [StmtSemi] which doesn't override
    /// [needs_semi][StmtSemi::needs_semi].
    pub(crate) fn new(expr: ast::Expr, semi_token: T![;]) -> Self {
        Self { expr, semi_token }
    }
}
