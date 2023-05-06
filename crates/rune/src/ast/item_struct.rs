use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ItemStruct>("struct Foo");
    rt::<ast::ItemStruct>("struct Foo ( a, b, c )");
    rt::<ast::ItemStruct>("struct Foo { a, b, c }");
    rt::<ast::ItemStruct>("struct Foo { #[default_value = 1] a, b, c }");
    rt::<ast::ItemStruct>("#[alpha] struct Foo ( #[default_value = \"x\" ] a, b, c )");

    rt::<ast::ItemStructBody>("");

    rt::<ast::ItemStructBody>("{ a, b, c }");
    rt::<ast::ItemStructBody>("{ #[x] a, #[y] b, #[z] #[w] #[f32] c }");
    rt::<ast::ItemStructBody>("{ a, #[attribute] b, c }");

    rt::<ast::ItemStructBody>("( a, b, c )");
    rt::<ast::ItemStructBody>("( #[x] a, b, c )");
    rt::<ast::ItemStructBody>("( #[x] pub a, b, c )");
    rt::<ast::ItemStructBody>("( a, b, c )");
    rt::<ast::ItemStructBody>("()");

    rt::<ast::Field>("a");
    rt::<ast::Field>("#[x] a");
}

/// A struct item.
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned, Opaque)]
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
    #[rune(optional, meta)]
    pub visibility: ast::Visibility,
    /// The `struct` keyword.
    pub struct_token: T![struct],
    /// The identifier of the struct declaration.
    pub ident: ast::Ident,
    /// The body of the struct.
    #[rune(optional)]
    pub body: ItemStructBody,
}

impl ItemStruct {
    /// If the struct declaration needs to be terminated with a semicolon.
    pub(crate) fn needs_semi_colon(&self) -> bool {
        self.body.needs_semi_colon()
    }
}

item_parse!(Struct, ItemStruct, "struct item");

/// AST for a struct body.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, OptionSpanned)]
#[non_exhaustive]
pub enum ItemStructBody {
    /// An empty struct declaration.
    UnitBody,
    /// A tuple struct body.
    TupleBody(ast::Parenthesized<Field, T![,]>),
    /// A regular struct body.
    StructBody(ast::Braced<Field, T![,]>),
}

impl ItemStructBody {
    /// If the body needs to be terminated with a semicolon.
    fn needs_semi_colon(&self) -> bool {
        matches!(self, Self::UnitBody | Self::TupleBody(..))
    }

    /// Iterate over the fields of the body.
    pub(crate) fn fields(&self) -> impl Iterator<Item = &'_ (Field, Option<T![,]>)> {
        match self {
            ItemStructBody::UnitBody => IntoIterator::into_iter(&[]),
            ItemStructBody::TupleBody(body) => body.iter(),
            ItemStructBody::StructBody(body) => body.iter(),
        }
    }
}

impl Parse for ItemStructBody {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        Ok(match p.nth(0)? {
            K!['('] => Self::TupleBody(p.parse()?),
            K!['{'] => Self::StructBody(p.parse()?),
            _ => Self::UnitBody,
        })
    }
}

/// A field as part of a struct or a tuple body.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
#[non_exhaustive]
pub struct Field {
    /// Attributes associated with field.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the field
    #[rune(optional)]
    pub visibility: ast::Visibility,
    /// Name of the field.
    pub name: ast::Ident,
}
