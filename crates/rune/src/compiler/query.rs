use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::compiler::index::{FunctionIndexer, Index as _};
use crate::compiler::Items;
use crate::error::CompileError;
use crate::source::Source;
use crate::traits::Resolve as _;
use runestick::{CompilationUnit, Component, Item, Meta, MetaObject, MetaTuple, Span};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

pub(super) enum Entry {
    Enum,
    Struct(Struct),
    Variant(Variant),
    Function(Function),
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

pub(super) struct Function {
    /// Ast for declaration.
    pub(super) ast: ast::DeclFn,
}

impl Function {
    /// Construct a new function.
    pub(super) fn new(ast: ast::DeclFn) -> Self {
        Self { ast }
    }
}

pub struct Query<'a> {
    pub(super) source: Source<'a>,
    pub(super) functions: VecDeque<(Item, Function)>,
    items: HashMap<Item, Entry>,
    pub(super) unit: Rc<RefCell<CompilationUnit>>,
}

impl<'a> Query<'a> {
    /// Construct a new compilation context.
    pub fn new(source: Source<'a>, unit: Rc<RefCell<CompilationUnit>>) -> Self {
        Self {
            source,
            functions: VecDeque::new(),
            items: HashMap::new(),
            unit,
        }
    }

    /// Process a single declaration.
    pub fn process_decl(&mut self, decl: ast::Decl) -> Result<(), CompileError> {
        match decl {
            ast::Decl::DeclUse(import) => {
                let name = import.path.resolve(self.source)?;
                self.unit.borrow_mut().new_import(Item::empty(), &name)?;
            }
            ast::Decl::DeclEnum(en) => {
                let name = en.name.resolve(self.source)?;
                let enum_item = Item::of(&[name]);
                self.new_enum(enum_item.clone());

                for (variant, body, _) in en.variants {
                    let variant = variant.resolve(self.source)?;
                    let item = Item::of(&[name, variant]);
                    self.new_variant(item, enum_item.clone(), body);
                }
            }
            ast::Decl::DeclStruct(st) => {
                let name = st.ident.resolve(self.source)?;
                let item = Item::of(&[name]);
                self.new_struct(item, st);
            }
            ast::Decl::DeclFn(f) => {
                let name = f.name.resolve(self.source)?;

                let items = Items::new(vec![Component::from(name)]);
                let item = items.item();

                let mut indexer = FunctionIndexer { items, query: self };

                indexer.index(&f)?;
                self.functions.push_back((item, Function::new(f)));
            }
        }

        Ok(())
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

    /// Add a new function that can be queried for.
    pub fn new_function(&mut self, item: Item, ast: ast::DeclFn) {
        self.items.insert(item, Entry::Function(Function::new(ast)));
    }

    /// Query for the given meta item.
    pub fn query_meta(&mut self, item: &Item, span: Span) -> Result<Option<Meta>, CompileError> {
        let item = Item::of(item);

        if let Some(meta) = self.unit.borrow().lookup_meta(&item) {
            return Ok(Some(meta));
        }

        let entry = match self.items.remove(&item) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        match entry {
            Entry::Enum => {
                self.unit
                    .borrow_mut()
                    .new_item(Meta::MetaEnum { item: item.clone() })?;
            }
            Entry::Variant(variant) => {
                // Assert that everything is built for the enum.
                self.query_meta(&variant.enum_item, span)?;

                let meta = self.ast_into_item_decl(&item, variant.ast, Some(variant.enum_item))?;
                self.unit.borrow_mut().new_item(meta)?;
            }
            Entry::Struct(st) => {
                let meta = self.ast_into_item_decl(&item, st.ast.body, None)?;
                self.unit.borrow_mut().new_item(meta)?;
            }
            Entry::Function(f) => {
                self.functions.push_back((item.clone(), f));
                self.unit
                    .borrow_mut()
                    .new_item(Meta::MetaFunction { item: item.clone() })?;
            }
        }

        match self.unit.borrow().lookup_meta(&item) {
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
