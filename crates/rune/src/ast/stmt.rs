use crate::ast::prelude::*;
use std::mem::take;

/// A statement within a block.
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Stmt>("let x = 1;");
/// testing::roundtrip::<ast::Stmt>("#[attr] let a = f();");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum Stmt {
    /// A local declaration.
    Local(Box<ast::Local>),
    /// A declaration.
    Item(ast::Item, #[rune(iter)] Option<T![;]>),
    /// An expression.
    Expr(ast::Expr, #[rune(iter)] Option<T![;]>),
}

impl Stmt {
    /// Get the sort key for the statement.
    ///
    /// This allows a collection of statements to be reordered into:
    /// * Uses
    /// * Items
    /// * Macro expansions.
    /// * The rest, expressions, local decl, etc...
    ///
    /// Note that the sort implementation must be stable, to make sure that
    /// intermediate items are not affected.
    pub fn sort_key(&self) -> StmtSortKey {
        match self {
            Stmt::Item(item, _) => match item {
                ast::Item::Use(_) => StmtSortKey::Use,
                ast::Item::MacroCall(_) => StmtSortKey::Other,
                _ => StmtSortKey::Item,
            },
            _ => StmtSortKey::Other,
        }
    }
}

impl Peek for Stmt {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K![let]) || ItemOrExpr::peek(p)
    }
}

impl Parse for Stmt {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
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
            return Err(ParseError::unsupported(span, "visibility modifier"));
        }

        let stmt = if let K![let] = p.nth(0)? {
            let local = Box::new(ast::Local::parse_with_meta(p, take(&mut attributes))?);
            Self::Local(local)
        } else {
            let expr = ast::Expr::parse_with_meta(p, &mut attributes, ast::expr::Callable(true))?;

            // Parsed an expression which can be treated directly as an item.
            match expr.into_item() {
                Ok(item) => Self::Item(item, p.parse()?),
                Err(expr) => Self::Expr(expr, p.parse()?),
            }
        };

        if let Some(span) = attributes.option_span() {
            return Err(ParseError::unsupported(span, "attributes"));
        }

        Ok(stmt)
    }
}

/// Parsing an item or an expression.
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
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        let mut attributes = p.parse()?;
        let visibility = p.parse()?;

        if ast::Item::peek_as_item(p.peeker()) {
            let path = p.parse()?;
            let item: ast::Item = ast::Item::parse_with_meta_path(p, attributes, visibility, path)?;
            return Ok(Self::Item(item));
        }

        if let Some(span) = visibility.option_span() {
            return Err(ParseError::unsupported(span, "visibility modifier"));
        }

        let expr = ast::Expr::parse_with_meta(p, &mut attributes, ast::expr::Callable(true))?;

        if let Some(span) = attributes.option_span() {
            return Err(ParseError::unsupported(span, "attributes"));
        }

        Ok(Self::Expr(expr))
    }
}

/// Key used to stort a statement into its processing order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StmtSortKey {
    /// USe statements, that should be processed first.
    Use,
    /// Items.
    Item,
    /// Other things, that should be processed last.
    Other,
}

#[cfg(test)]
mod tests {
    use crate::ast;
    use crate::testing::roundtrip;

    #[test]
    fn test_stmt_local() {
        roundtrip::<ast::Stmt>("let x = 1;");
        roundtrip::<ast::Stmt>("#[attr] let a = f();");
    }

    #[test]
    fn test_macro_call_chain() {
        roundtrip::<ast::Stmt>("line!().bar()");
    }
}
