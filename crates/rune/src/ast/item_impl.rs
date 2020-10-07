use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// An impl item.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ItemImpl>("impl Foo {}");
/// testing::roundtrip::<ast::ItemImpl>("impl Foo { fn test(self) { } }");
/// testing::roundtrip::<ast::ItemImpl>("#[variant(enum_= \"SuperHero\", x = \"1\")] impl Foo { fn test(self) { } }");
/// testing::roundtrip::<ast::ItemImpl>("#[xyz] impl Foo { #[jit] fn test(self) { } }");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ItemImpl {
    /// The attributes of the `impl` block
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The `impl` keyword.
    pub impl_: T![impl],
    /// Path of the implementation.
    pub path: ast::Path,
    /// The open brace.
    pub open: T!['{'],
    /// The collection of functions.
    pub functions: Vec<ast::ItemFn>,
    /// The close brace.
    pub close: T!['}'],
}

impl ItemImpl {
    /// Parse an `impl` item with the given attributes.
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let impl_ = parser.parse()?;
        let path = parser.parse()?;
        let open = parser.parse()?;

        let mut functions = vec![];

        while !parser.peek::<ast::CloseBrace>()? {
            functions.push(ast::ItemFn::parse(parser)?);
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

item_parse!(ItemImpl, "impl item");
