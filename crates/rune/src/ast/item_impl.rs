use crate::ast::prelude::*;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
    rt::<ast::ItemImpl>("impl Foo {}");
    rt::<ast::ItemImpl>("impl Foo { fn test(self) { } }");
    rt::<ast::ItemImpl>(
        "#[variant(enum_= \"SuperHero\", x = \"1\")] impl Foo { fn test(self) { } }",
    );
    rt::<ast::ItemImpl>("#[xyz] impl Foo { #[jit] fn test(self) { } }");
    rt::<ast::ItemImpl>("impl Foo { const X = 1; }");
    rt::<ast::ItemImpl>("impl Foo { pub const X = 1; fn x() { Self::X } const Y = 2; }");
    rt::<ast::ItemImpl>("impl Foo { /// Docs.\n #[doc = \"y\"] pub const X = Self(1); }");
    rt::<ast::ItemImpl>("impl Foo { pub const fn x() { 1 } }");

    let item = rt::<ast::ItemImpl>("impl Foo { const X = 1; fn x() {} }");
    assert!(matches!(
        item.items.as_slice(),
        [
            (ast::ImplItem::Const(..), Some(..)),
            (ast::ImplItem::Fn(..), None)
        ]
    ));
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
    /// The functions and constants of the impl, in the order they were
    /// written, each with the semicolon which followed it if any. A constant
    /// is always followed by one.
    #[rune(iter)]
    pub items: Vec<(ast::ImplItem, Option<T![;]>)>,
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

        let mut items = Vec::new();

        while !parser.peek::<ast::CloseBrace>()? {
            let item = ast::ImplItem::parse(parser)?;

            let semi_colon =
                if matches!(item, ast::ImplItem::Const(..)) || parser.peek::<T![;]>()? {
                    Some(parser.parse::<T![;]>()?)
                } else {
                    None
                };

            items.try_push((item, semi_colon))?;
        }

        let close = parser.parse()?;

        Ok(Self {
            attributes,
            impl_,
            path,
            open,
            items,
            close,
        })
    }
}

item_parse!(Impl, ItemImpl, "impl item");

/// An item inside of an `impl` block.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum ImplItem {
    /// An associated function.
    Fn(ast::ItemFn),
    /// An associated constant.
    Const(ast::ItemConst),
}

impl Parse for ImplItem {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        let attributes = p.parse()?;
        let visibility = p.parse()?;
        let const_token = p.parse::<Option<T![const]>>()?;

        // `const` is both what starts a constant and a modifier on a function,
        // so which one this is is only known from what follows it.
        if let (Some(const_token), K![ident]) = (&const_token, p.nth(0)?) {
            return Ok(Self::Const(ast::ItemConst::parse_with_meta(
                p,
                attributes,
                visibility,
                *const_token,
            )?));
        }

        let async_token = p.parse()?;

        Ok(Self::Fn(ast::ItemFn::parse_with_meta(
            p,
            attributes,
            visibility,
            const_token,
            async_token,
        )?))
    }
}
