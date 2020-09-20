use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// An impl declaration.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ItemImpl {
    /// The attributes of the `impl` block
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
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

impl ItemImpl {
    /// Parse an `impl` item with the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let impl_ = parser.parse()?;
        let path = parser.parse()?;
        let open = parser.parse()?;

        let mut functions = vec![];

        while !parser.peek::<ast::CloseBrace>()? {
            let attributes = parser.parse()?;
            functions.push(ast::ItemFn::parse_with_attributes(parser, attributes)?);
        }

        let close = parser.parse()?;

        Ok(Self {
            attributes,
            impl_,
            path,
            open,
            functions,
            close,
        })
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
/// parse_all::<ast::ItemImpl>("#[variant(enum_= \"SuperHero\", x = \"1\")] impl Foo { fn test(self) { } }").unwrap();
/// parse_all::<ast::ItemImpl>("#[xyz] impl Foo { #[jit] fn test(self) { } }").unwrap();
/// ```
impl Parse for ItemImpl {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}
