use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::error::CompileError;
use crate::source::Source;
use crate::traits::Resolve as _;
use runestick::{CompilationUnit, Item, Meta, MetaTuple, MetaType, Span};

pub enum Entry {
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
    ast: ast::DeclStructBody,
}

impl Variant {
    /// Construct a new variant.
    pub fn new(ast: ast::DeclStructBody) -> Self {
        Self { ast }
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

    /// Add a new struct item that can be queried.
    pub fn new_struct(&mut self, item: Item, ast: ast::DeclStruct) {
        self.items.insert(item, Entry::Struct(Struct::new(ast)));
    }

    /// Add a new variant item that can be queried.
    pub fn new_variant(&mut self, item: Item, ast: ast::DeclStructBody) {
        self.items.insert(item, Entry::Variant(Variant::new(ast)));
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
            Entry::Variant(variant) => {
                let meta = self.ast_into_item_decl(&item, variant.ast)?;
                unit.new_item(&item, meta)?;
            }
            Entry::Struct(st) => {
                let meta = self.ast_into_item_decl(&item, st.ast.body)?;
                unit.new_item(&item, meta)?;
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
    ) -> Result<Meta, CompileError> {
        Ok(match body {
            ast::DeclStructBody::EmptyBody(..) => {
                let meta = Meta::MetaTuple(MetaTuple {
                    external: false,
                    item: item.clone(),
                    args: 0,
                });

                meta
            }
            ast::DeclStructBody::TupleBody(tuple) => {
                let meta = Meta::MetaTuple(MetaTuple {
                    external: false,
                    item: item.clone(),
                    args: tuple.fields.len(),
                });

                meta
            }
            ast::DeclStructBody::StructBody(st) => {
                let mut fields = HashSet::new();

                for (ident, _) in &st.fields {
                    let ident = ident.resolve(self.source)?;
                    fields.insert(ident.to_owned());
                }

                let meta = Meta::MetaType(MetaType {
                    item: item.clone(),
                    fields,
                });

                let mut fields = HashMap::new();

                for (index, (ident, _)) in st.fields.iter().enumerate() {
                    let ident = ident.resolve(self.source)?;
                    fields.insert(ident.to_owned(), index);
                }

                meta
            }
        })
    }
}
