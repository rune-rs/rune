use crate::ast;
use crate::{IntoTokens, Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// An impl declaration.
#[derive(Debug, Clone)]
pub struct ItemImpl {
    /// The `impl` keyword.
    pub impl_: ast::Impl,
    /// Path of the implementation.
    pub path: ast::Path,
    /// The open brace.
    pub open: ast::OpenBrace,
    /// The collection of functions.
    pub functions: Vec<ast::ItemFn>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl Spanned for ItemImpl {
    fn span(&self) -> Span {
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
/// parse_all::<ast::ItemImpl>("impl Foo {}").unwrap();
/// parse_all::<ast::ItemImpl>("impl Foo { fn test(self) { } }").unwrap();
/// ```
impl Parse for ItemImpl {
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

impl IntoTokens for ItemImpl {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.impl_.into_tokens(context, stream);
        self.path.into_tokens(context, stream);
        self.open.into_tokens(context, stream);
        self.functions.into_tokens(context, stream);
        self.close.into_tokens(context, stream);
    }
}
