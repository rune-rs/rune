use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::error::CompileError;
use crate::source::Source;
use crate::traits::Resolve as _;
use runestick::{CompilationUnit, Item, Meta, MetaObject, MetaTuple, Span};

pub enum Entry {
    Enum,
    Struct(Struct),
    Variant(Variant),
}

pub struct Struct {
    ast: ast::DeclStruct,
}

impl Struct {
    /// Construct a new struct entry.
    pub fn new(ast: ast::DeclStruct) -> Self {
        Self { ast }
    }
}

pub struct Variant {
    /// Item of the enum type.
    enum_item: Item,
    /// Ast for declaration.
    ast: ast::DeclStructBody,
}

impl Variant {
    /// Construct a new variant.
    pub fn new(enum_item: Item, ast: ast::DeclStructBody) -> Self {
        Self { enum_item, ast }
    }
}

pub struct Query<'a> {
    source: Source<'a>,
    items: HashMap<Item, Entry>,
}

impl<'a> Query<'a> {
    /// Construct a new compilation context.
    pub fn new(source: Source<'a>) -> Self {
        Self {
            source,
            items: HashMap::new(),
        }
    }

    /// Add a new enum item.
    pub fn new_enum(&mut self, item: Item) {
        self.items.insert(item, Entry::Enum);
    }

    /// Add a new struct item that can be queried.
    pub fn new_struct(&mut self, item: Item, ast: ast::DeclStruct) {
        self.items.insert(item, Entry::Struct(Struct::new(ast)));
    }

    /// Add a new variant item that can be queried.
    pub fn new_variant(&mut self, item: Item, enum_item: Item, ast: ast::DeclStructBody) {
        self.items
            .insert(item, Entry::Variant(Variant::new(enum_item, ast)));
    }

    /// Query for the given meta item.
    pub fn query_meta(
        &mut self,
        unit: &mut CompilationUnit,
        item: &Item,
        span: Span,
    ) -> Result<Option<Meta>, CompileError> {
        let item = Item::of(item);

        if let Some(meta) = unit.lookup_meta(&item) {
            return Ok(Some(meta));
        }

        let entry = match self.items.remove(&item) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        match entry {
            Entry::Enum => {
                unit.new_item(Meta::MetaEnum { item: item.clone() })?;
            }
            Entry::Variant(variant) => {
                // Assert that everything is built for the enum.
                self.query_meta(unit, &variant.enum_item, span)?;

                let meta = self.ast_into_item_decl(&item, variant.ast, Some(variant.enum_item))?;
                unit.new_item(meta)?;
            }
            Entry::Struct(st) => {
                let meta = self.ast_into_item_decl(&item, st.ast.body, None)?;
                unit.new_item(meta)?;
            }
        }

        match unit.lookup_meta(&item) {
            Some(meta) => Ok(Some(meta)),
            None => Err(CompileError::MissingType { span, item }),
        }
    }

    /// Convert an ast declaration into a struct.
    fn ast_into_item_decl(
        &self,
        item: &Item,
        body: ast::DeclStructBody,
        enum_item: Option<Item>,
    ) -> Result<Meta, CompileError> {
        Ok(match body {
            ast::DeclStructBody::EmptyBody(..) => {
                let tuple = MetaTuple {
                    item: item.clone(),
                    args: 0,
                };

                match enum_item {
                    Some(enum_item) => Meta::MetaTupleVariant { enum_item, tuple },
                    None => Meta::MetaTuple { tuple },
                }
            }
            ast::DeclStructBody::TupleBody(tuple) => {
                let tuple = MetaTuple {
                    item: item.clone(),
                    args: tuple.fields.len(),
                };

                match enum_item {
                    Some(enum_item) => Meta::MetaTupleVariant { enum_item, tuple },
                    None => Meta::MetaTuple { tuple },
                }
            }
            ast::DeclStructBody::StructBody(st) => {
                let mut fields = HashSet::new();

                for (ident, _) in &st.fields {
                    let ident = ident.resolve(self.source)?;
                    fields.insert(ident.to_owned());
                }

                let object = MetaObject {
                    item: item.clone(),
                    fields: Some(fields),
                };

                match enum_item {
                    Some(enum_item) => Meta::MetaObjectVariant { enum_item, object },
                    None => Meta::MetaObject { object },
                }
            }
        })
    }
}
