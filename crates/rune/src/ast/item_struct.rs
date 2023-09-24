use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ItemStruct>("struct Foo");
    rt::<ast::ItemStruct>("struct Foo ( a, b, c )");
    rt::<ast::ItemStruct>("struct Foo { a, b, c }");
    rt::<ast::ItemStruct>("struct Foo { #[default_value = 1] a, b, c }");
    rt::<ast::ItemStruct>("#[alpha] struct Foo ( #[default_value = \"x\" ] a, b, c )");

    rt::<ast::Fields>("");

    rt::<ast::Fields>("{ a, b, c }");
    rt::<ast::Fields>("{ #[x] a, #[y] b, #[z] #[w] #[f32] c }");
    rt::<ast::Fields>("{ a, #[attribute] b, c }");

    rt::<ast::Fields>("( a, b, c )");
    rt::<ast::Fields>("( #[x] a, b, c )");
    rt::<ast::Fields>("( #[x] pub a, b, c )");
    rt::<ast::Fields>("( a, b, c )");
    rt::<ast::Fields>("()");

    rt::<ast::Field>("a");
    rt::<ast::Field>("#[x] a");
}

/// A struct item.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned, Opaque)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ItemStruct {
    /// Opaque identifier of the struct.
    #[rune(id)]
    pub(crate) id: Id,
    /// The attributes for the struct
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `struct` item
    #[rune(option, meta)]
    pub visibility: ast::Visibility,
    /// The `struct` keyword.
    pub struct_token: T![struct],
    /// The identifier of the struct declaration.
    pub ident: ast::Ident,
    /// The body of the struct.
    #[rune(iter)]
    pub body: ast::Fields,
}

impl ItemStruct {
    /// If the struct declaration needs to be terminated with a semicolon.
    pub(crate) fn needs_semi_colon(&self) -> bool {
        self.body.needs_semi_colon()
    }
}

item_parse!(Struct, ItemStruct, "struct item");

/// A field as part of a struct or a tuple body.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Parse, Spanned)]
#[non_exhaustive]
pub struct Field {
    /// Attributes associated with field.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the field
    #[rune(option)]
    pub visibility: ast::Visibility,
    /// Name of the field.
    pub name: ast::Ident,
}
