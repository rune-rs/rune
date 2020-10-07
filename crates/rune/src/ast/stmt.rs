use crate::ast;
use crate::{OptionSpanned as _, Parse, ParseError, Parser, Peek, Peeker, Spanned, ToTokens};
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
    Expr(ast::Expr),
    /// An expression followed by a semicolon.
    Semi(ast::Expr, T![;]),
}

impl Peek for Stmt {
    fn peek(p: &mut Peeker<'_>) -> bool {
        match [p.nth(0), p.nth(1)] {
            [K![let], ..] => true,
            [K![use], ..] => true,
            [K![enum], ..] => true,
            [K![struct], ..] => true,
            [K![impl], ..] => true,
            [K![async], K![fn]] => true,
            [K![fn], ..] => true,
            [K![mod], ..] => true,
            [K![const], ..] => true,
            [ast::Kind::Ident { .. }, ..] => true,
            _ => ast::Expr::peek(p),
        }
    }
}

impl Parse for Stmt {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        let mut attributes = p.parse()?;
        let visibility = p.parse()?;
        let path = p.parse::<Option<ast::Path>>()?;

        if ast::Item::peek_as_item(p.peeker(), path.as_ref()) {
            let item: ast::Item = ast::Item::parse_with_meta_path(p, attributes, visibility, path)?;

            let semi = if item.needs_semi_colon() {
                Some(p.parse()?)
            } else {
                p.parse()?
            };

            return Ok(ast::Stmt::Item(item, semi));
        }

        if let Some(span) = visibility.option_span() {
            return Err(ParseError::unsupported(span, "visibility modifier"));
        }

        let stmt = if let K![let] = p.nth(0)? {
            if let Some(path) = path {
                return Err(ParseError::expected(&path.first, "expected let statement"));
            }

            let local = Box::new(ast::Local::parse_with_meta(p, take(&mut attributes))?);
            ast::Stmt::Local(local)
        } else {
            let expr =
                ast::Expr::parse_with_meta(p, &mut attributes, path, ast::expr::Callable(false))?;

            if p.peek::<T![;]>()? {
                ast::Stmt::Semi(expr, p.parse()?)
            } else {
                ast::Stmt::Expr(expr)
            }
        };

        if let Some(span) = attributes.option_span() {
            return Err(ParseError::unsupported(span, "attributes"));
        }

        Ok(stmt)
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast, testing};

    #[test]
    fn test_stmt_local() {
        testing::roundtrip::<ast::Stmt>("let x = 1;");
        testing::roundtrip::<ast::Stmt>("#[attr] let a = f();");
    }
}
