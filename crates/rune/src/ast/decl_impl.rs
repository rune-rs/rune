use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::Span;

/// An impl declaration.
#[derive(Debug, Clone)]
pub struct DeclImpl {
    /// The `impl` keyword.
    pub impl_: ast::Impl,
    /// Path of the implementation.
    pub path: ast::Path,
    /// The open brace.
    pub open: ast::OpenBrace,
    /// The collection of functions.
    pub functions: Vec<ast::DeclFn>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl DeclImpl {
    /// The span of the declaration.
    pub fn span(&self) -> Span {
        self.impl_.span().join(self.close.span())
    }
}

/// Parse implementation for an impl.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::DeclImpl>("impl Foo {}").unwrap();
/// parse_all::<ast::DeclImpl>("impl Foo { fn test(self) { } }").unwrap();
/// ```
impl Parse for DeclImpl {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            impl_: parser.parse()?,
            path: parser.parse()?,
            open: parser.parse()?,
            functions: parser.parse()?,
            close: parser.parse()?,
        })
    }
}
