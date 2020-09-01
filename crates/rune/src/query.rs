//! Lazy query system, used to compile and build items on demand.

use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::error::CompileError;
use crate::source::Source;
use crate::traits::Resolve as _;
use runestick::{
    Hash, Item, Meta, MetaClosureCapture, MetaStruct, MetaTuple, Span, Unit, ValueType,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

pub(super) enum Indexed {
    Enum,
    Struct(Struct),
    Variant(Variant),
    Function(Function),
    Closure(Closure),
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

pub(super) struct InstanceFunction {
    /// Ast for the instance function.
    pub(super) ast: ast::DeclFn,
    /// The item of the instance function.
    pub(super) instance_item: Item,
    /// The span of the instance function.
    pub(super) instance_span: Span,
}

pub(super) struct Closure {
    /// Ast for closure.
    pub(super) ast: ast::ExprClosure,
    /// Captures.
    pub(super) captures: Arc<Vec<MetaClosureCapture>>,
}

/// An entry in the build queue.
pub(super) enum Build {
    Function(Function),
    InstanceFunction(InstanceFunction),
    Closure(Closure),
}

pub(super) struct Query<'a> {
    pub(super) source: Source<'a>,
    pub(super) queue: VecDeque<(Item, Build)>,
    indexed: HashMap<Item, Indexed>,
    pub(super) unit: Rc<RefCell<Unit>>,
}

impl<'a> Query<'a> {
    /// Construct a new compilation context.
    pub fn new(source: Source<'a>, unit: Rc<RefCell<Unit>>) -> Self {
        Self {
            source,
            queue: VecDeque::new(),
            indexed: HashMap::new(),
            unit,
        }
    }

    /// Add a new enum item.
    pub fn index_enum(&mut self, item: Item, span: Span) -> Result<(), CompileError> {
        log::trace!("new enum: {}", item);
        self.index(item, Indexed::Enum, span)?;
        Ok(())
    }

    /// Add a new struct item that can be queried.
    pub fn index_struct(&mut self, item: Item, ast: ast::DeclStruct) -> Result<(), CompileError> {
        log::trace!("new struct: {}", item);
        let span = ast.span();
        self.index(item, Indexed::Struct(Struct::new(ast)), span)?;
        Ok(())
    }

    /// Add a new variant item that can be queried.
    pub fn index_variant(
        &mut self,
        item: Item,
        enum_item: Item,
        ast: ast::DeclStructBody,
        span: Span,
    ) -> Result<(), CompileError> {
        log::trace!("new variant: {}", item);
        self.index(item, Indexed::Variant(Variant::new(enum_item, ast)), span)?;
        Ok(())
    }

    /// Add a new function that can be queried for.
    pub fn index_closure(
        &mut self,
        item: Item,
        ast: ast::ExprClosure,
        captures: Arc<Vec<MetaClosureCapture>>,
    ) -> Result<(), CompileError> {
        let span = ast.span();
        log::trace!("new closure: {}", item);
        self.index(item, Indexed::Closure(Closure { ast, captures }), span)?;
        Ok(())
    }

    /// Index the given element.
    pub fn index(&mut self, item: Item, indexed: Indexed, span: Span) -> Result<(), CompileError> {
        log::trace!("indexed: {}", item);

        if let Some(..) = self.indexed.insert(item.clone(), indexed) {
            return Err(CompileError::ItemConflict {
                existing: item,
                span,
            });
        }

        Ok(())
    }

    /// Query for the given meta item.
    pub fn query_meta(&mut self, item: &Item, span: Span) -> Result<Option<Meta>, CompileError> {
        let item = Item::of(item);

        if let Some(meta) = self.unit.borrow().lookup_meta(&item) {
            return Ok(Some(meta));
        }

        // See if there's an index entry we can construct.
        let entry = match self.indexed.remove(&item) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        let meta = match entry {
            Indexed::Enum => Meta::MetaEnum {
                value_type: ValueType::Type(Hash::type_hash(&item)),
                item: item.clone(),
            },
            Indexed::Variant(variant) => {
                // Assert that everything is built for the enum.
                self.query_meta(&variant.enum_item, span)?;
                self.ast_into_item_decl(&item, variant.ast, Some(variant.enum_item))?
            }
            Indexed::Struct(st) => self.ast_into_item_decl(&item, st.ast.body, None)?,
            Indexed::Function(f) => {
                self.queue.push_back((item.clone(), Build::Function(f)));

                Meta::MetaFunction {
                    value_type: ValueType::Type(Hash::type_hash(&item)),
                    item: item.clone(),
                }
            }
            Indexed::Closure(c) => {
                let captures = c.captures.clone();
                self.queue.push_back((item.clone(), Build::Closure(c)));

                Meta::MetaClosure {
                    value_type: ValueType::Type(Hash::type_hash(&item)),
                    item: item.clone(),
                    captures,
                }
            }
        };

        self.unit.borrow_mut().insert_meta(meta)?;

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
        let value_type = ValueType::Type(Hash::type_hash(item));

        Ok(match body {
            ast::DeclStructBody::EmptyBody(..) => {
                let tuple = MetaTuple {
                    item: item.clone(),
                    args: 0,
                    hash: Hash::type_hash(item),
                };

                match enum_item {
                    Some(enum_item) => Meta::MetaVariantTuple {
                        value_type,
                        enum_item,
                        tuple,
                    },
                    None => Meta::MetaTuple { value_type, tuple },
                }
            }
            ast::DeclStructBody::TupleBody(tuple) => {
                let tuple = MetaTuple {
                    item: item.clone(),
                    args: tuple.fields.len(),
                    hash: Hash::type_hash(item),
                };

                match enum_item {
                    Some(enum_item) => Meta::MetaVariantTuple {
                        value_type,
                        enum_item,
                        tuple,
                    },
                    None => Meta::MetaTuple { value_type, tuple },
                }
            }
            ast::DeclStructBody::StructBody(st) => {
                let mut fields = HashSet::new();

                for (ident, _) in &st.fields {
                    let ident = ident.resolve(self.source)?;
                    fields.insert(ident.to_owned());
                }

                let object = MetaStruct {
                    item: item.clone(),
                    fields: Some(fields),
                };

                match enum_item {
                    Some(enum_item) => Meta::MetaVariantStruct {
                        value_type,
                        enum_item,
                        object,
                    },
                    None => Meta::MetaStruct { value_type, object },
                }
            }
        })
    }
}
