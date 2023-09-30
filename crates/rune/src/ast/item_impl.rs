use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ItemImpl>("impl Foo {}");
    rt::<ast::ItemImpl>("impl Foo { fn test(self) { } }");
    rt::<ast::ItemImpl>(
        "#[variant(enum_= \"SuperHero\", x = \"1\")] impl Foo { fn test(self) { } }",
    );
    rt::<ast::ItemImpl>("#[xyz] impl Foo { #[jit] fn test(self) { } }");
}

/// An impl item.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
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
    #[rune(iter)]
    pub functions: Vec<ast::ItemFn>,
    /// The close brace.
    pub close: T!['}'],
}

impl ItemImpl {
    /// Parse an `impl` item with the given attributes.
    pub(crate) fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self> {
        let impl_ = parser.parse()?;
        let path = parser.parse()?;
        let open = parser.parse()?;

        let mut functions = Vec::new();

        while !parser.peek::<ast::CloseBrace>()? {
            functions.try_push(ast::ItemFn::parse(parser)?)?;
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

item_parse!(Impl, ItemImpl, "impl item");
