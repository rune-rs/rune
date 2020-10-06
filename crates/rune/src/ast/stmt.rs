use crate::ast;
use crate::{
    OptionSpanned as _, Parse, ParseError, ParseErrorKind, Parser, Peek, Spanned, ToTokens,
};
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
    Local(ast::Local),
    /// A declaration.
    Item(ast::Item, #[rune(iter)] Option<ast::SemiColon>),
    /// An expression.
    Expr(ast::Expr),
    /// An expression followed by a semicolon.
    Semi(ast::Expr, ast::SemiColon),
}

impl Peek for Stmt {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        match peek!(t1).kind {
            ast::Kind::Let => true,
            ast::Kind::Use => true,
            ast::Kind::Enum => true,
            ast::Kind::Struct => true,
            ast::Kind::Impl => true,
            ast::Kind::Async => matches!(peek!(t2).kind, ast::Kind::Fn),
            ast::Kind::Fn => true,
            ast::Kind::Mod => true,
            ast::Kind::Const => true,
            ast::Kind::Ident { .. } => true,
            _ => ast::Expr::peek(t1, t2),
        }
    }
}

impl Parse for Stmt {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let mut attributes = parser.parse()?;
        let visibility = parser.parse()?;
        let path = parser.parse::<Option<ast::Path>>()?;

        if ast::Item::peek_as_item(parser, path.as_ref())? {
            let item: ast::Item =
                ast::Item::parse_with_meta_path(parser, attributes, visibility, path)?;

            let semi = if item.needs_semi_colon() {
                Some(parser.parse()?)
            } else {
                parser.parse()?
            };

            return Ok(ast::Stmt::Item(item, semi));
        }

        if let Some(span) = visibility.option_span() {
            return Err(ParseError::new(
                span,
                ParseErrorKind::UnsupportedExprVisibility,
            ));
        }

        let stmt = if let ast::Kind::Let = parser.token_peek_eof()?.kind {
            if let Some(path) = path {
                return Err(ParseError::expected(path.first, "expected let statement"));
            }

            let local = ast::Local::parse_with_meta(parser, take(&mut attributes))?;
            ast::Stmt::Local(local)
        } else {
            let expr = ast::Expr::parse_with_meta(parser, &mut attributes, path)?;

            if parser.peek::<ast::SemiColon>()? {
                ast::Stmt::Semi(expr, parser.parse()?)
            } else {
                ast::Stmt::Expr(expr)
            }
        };

        if let Some(span) = attributes.option_span() {
            return Err(ParseError::new(
                span,
                ParseErrorKind::AttributesNotSupported,
            ));
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
