//! A single execution unit in the runestick virtual machine.
//!
//! A unit consists of a sequence of instructions, and lookaside tables for
//! metadata like function locations.

use crate::ast;
use crate::collections::HashMap;
use crate::compiling::{Assembly, AssemblyInst};
use crate::query::QueryMod;
use crate::CompileResult;
use crate::{CompileError, CompileErrorKind, Error, Errors, Resolve as _, Spanned, Storage};
use runestick::debug::{DebugArgs, DebugSignature};
use runestick::{
    Call, CompileMeta, CompileMetaKind, Component, Context, DebugInfo, DebugInst, Hash, Inst,
    IntoComponent, Item, Label, Names, Rtti, Source, SourceId, Span, StaticString, Type, Unit,
    UnitFn, UnitTypeInfo, VariantRtti,
};
use std::cell::{Ref, RefCell};
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use thiserror::Error;

/// The key of an import.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportKey {
    /// Where the import is located.
    pub item: Item,
    /// The component that is imported.
    pub component: Component,
}

impl ImportKey {
    /// Construct a new import key.
    pub fn new<C>(item: Item, component: C) -> Self
    where
        C: IntoComponent,
    {
        Self {
            item,
            component: component.into_component(),
        }
    }

    /// Construct an import key for a single component.
    pub fn component<C>(component: C) -> Self
    where
        C: IntoComponent,
    {
        Self {
            item: Item::new(),
            component: component.into_component(),
        }
    }
}

impl fmt::Display for ImportKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.item.is_empty() {
            write!(f, "{}::", self.item)?;
        }

        write!(f, "{}", self.component)
    }
}

/// An imported entry.
#[derive(Debug, Clone)]
pub struct ImportEntry {
    /// The item being imported.
    pub item: Item,
    /// The span of the import.
    pub span: Option<(Span, SourceId)>,
}

impl ImportEntry {
    /// Construct an entry.
    pub fn of<I>(iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        Self {
            item: Item::of(iter),
            span: None,
        }
    }
}

/// Instructions from a single source file.
#[derive(Debug, Default, Clone)]
pub struct UnitBuilder {
    inner: Rc<RefCell<Inner>>,
}

impl UnitBuilder {
    /// Construct a new unit with the default prelude.
    pub fn with_default_prelude() -> Self {
        let mut this = Inner::default();

        this.imports.insert(
            ImportKey::component("dbg"),
            ImportEntry::of(&["std", "dbg"]),
        );
        this.imports.insert(
            ImportKey::component("drop"),
            ImportEntry::of(&["std", "drop"]),
        );
        this.imports.insert(
            ImportKey::component("is_readable"),
            ImportEntry::of(&["std", "is_readable"]),
        );
        this.imports.insert(
            ImportKey::component("is_writable"),
            ImportEntry::of(&["std", "is_writable"]),
        );
        this.imports.insert(
            ImportKey::component("panic"),
            ImportEntry::of(&["std", "panic"]),
        );
        this.imports.insert(
            ImportKey::component("print"),
            ImportEntry::of(&["std", "print"]),
        );
        this.imports.insert(
            ImportKey::component("println"),
            ImportEntry::of(&["std", "println"]),
        );
        this.imports.insert(
            ImportKey::component("unit"),
            ImportEntry::of(&["std", "unit"]),
        );
        this.imports.insert(
            ImportKey::component("bool"),
            ImportEntry::of(&["std", "bool"]),
        );
        this.imports.insert(
            ImportKey::component("byte"),
            ImportEntry::of(&["std", "byte"]),
        );
        this.imports.insert(
            ImportKey::component("char"),
            ImportEntry::of(&["std", "char"]),
        );
        this.imports.insert(
            ImportKey::component("int"),
            ImportEntry::of(&["std", "int"]),
        );
        this.imports.insert(
            ImportKey::component("float"),
            ImportEntry::of(&["std", "float"]),
        );
        this.imports.insert(
            ImportKey::component("Object"),
            ImportEntry::of(&["std", "object", "Object"]),
        );
        this.imports.insert(
            ImportKey::component("Vec"),
            ImportEntry::of(&["std", "vec", "Vec"]),
        );
        this.imports.insert(
            ImportKey::component("String"),
            ImportEntry::of(&["std", "string", "String"]),
        );
        this.imports.insert(
            ImportKey::component("Result"),
            ImportEntry::of(&["std", "result", "Result"]),
        );
        this.imports.insert(
            ImportKey::component("Err"),
            ImportEntry::of(&["std", "result", "Result", "Err"]),
        );
        this.imports.insert(
            ImportKey::component("Ok"),
            ImportEntry::of(&["std", "result", "Result", "Ok"]),
        );
        this.imports.insert(
            ImportKey::component("Option"),
            ImportEntry::of(&["std", "option", "Option"]),
        );
        this.imports.insert(
            ImportKey::component("Some"),
            ImportEntry::of(&["std", "option", "Option", "Some"]),
        );
        this.imports.insert(
            ImportKey::component("None"),
            ImportEntry::of(&["std", "option", "Option", "None"]),
        );

        Self {
            inner: Rc::new(RefCell::new(this)),
        }
    }

    /// Convert into a runtime unit, shedding our build metadata in the process.
    ///
    /// Returns `None` if the builder is still in use.
    pub fn build(self) -> Option<Unit> {
        let inner = Rc::try_unwrap(self.inner).ok()?;
        let mut inner = inner.into_inner();

        if let Some(debug) = &mut inner.debug {
            debug.functions_rev = inner.functions_rev;
        }

        Some(Unit::new(
            inner.instructions,
            inner.functions,
            inner.types,
            inner.static_strings,
            inner.static_bytes,
            inner.static_object_keys,
            inner.rtti,
            inner.variant_rtti,
            inner.debug,
        ))
    }

    /// Check if unit contains the given name by prefix.
    pub(crate) fn contains_prefix(&self, item: &Item) -> bool {
        self.inner.borrow().names.contains_prefix(item)
    }

    /// Access imports.
    pub(crate) fn imports(&self) -> Ref<'_, HashMap<ImportKey, ImportEntry>> {
        let inner = self.inner.borrow();
        Ref::map(inner, |inner| &inner.imports)
    }

    /// Iterate over known child components of the given name.
    pub(crate) fn iter_components<I>(&self, iter: I) -> Vec<Component>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let inner = self.inner.borrow();
        inner
            .names
            .iter_components(iter)
            .map(|c| c.into_component())
            .collect::<Vec<_>>()
    }

    /// Access the meta for the given language item.
    pub(crate) fn lookup_meta(&self, name: &Item) -> Option<CompileMeta> {
        self.inner.borrow().meta.get(name).cloned()
    }

    /// Insert a static string and return its associated slot that can later be
    /// looked up through [lookup_string][Self::lookup_string].
    ///
    /// Only uses up space if the static string is unique.
    pub(crate) fn new_static_string<S>(
        &self,
        spanned: S,
        current: &str,
    ) -> Result<usize, CompileError>
    where
        S: Copy + Spanned,
    {
        let mut inner = self.inner.borrow_mut();

        let current = StaticString::new(current);
        let hash = current.hash();

        if let Some(existing_slot) = inner.static_string_rev.get(&hash).copied() {
            let existing = inner.static_strings.get(existing_slot).ok_or_else(|| {
                CompileError::new(
                    spanned,
                    CompileErrorKind::StaticStringMissing {
                        hash,
                        slot: existing_slot,
                    },
                )
            })?;

            if ***existing != *current {
                return Err(CompileError::new(
                    spanned,
                    CompileErrorKind::StaticStringHashConflict {
                        hash,
                        current: (*current).clone(),
                        existing: (***existing).clone(),
                    },
                ));
            }

            return Ok(existing_slot);
        }

        let new_slot = inner.static_strings.len();
        inner.static_strings.push(Arc::new(current));
        inner.static_string_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    /// Insert a static byte string and return its associated slot that can
    /// later be looked up through [lookup_bytes][Self::lookup_bytes].
    ///
    /// Only uses up space if the static byte string is unique.
    pub(crate) fn new_static_bytes<S>(
        &self,
        spanned: S,
        current: &[u8],
    ) -> Result<usize, CompileError>
    where
        S: Copy + Spanned,
    {
        let mut inner = self.inner.borrow_mut();

        let hash = Hash::static_bytes(&current);

        if let Some(existing_slot) = inner.static_bytes_rev.get(&hash).copied() {
            let existing = inner.static_bytes.get(existing_slot).ok_or_else(|| {
                CompileError::new(
                    spanned,
                    CompileErrorKind::StaticBytesMissing {
                        hash,
                        slot: existing_slot,
                    },
                )
            })?;

            if &**existing != current {
                return Err(CompileError::new(
                    spanned,
                    CompileErrorKind::StaticBytesHashConflict {
                        hash,
                        current: current.to_owned(),
                        existing: existing.clone(),
                    },
                ));
            }

            return Ok(existing_slot);
        }

        let new_slot = inner.static_bytes.len();
        inner.static_bytes.push(current.to_owned());
        inner.static_bytes_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    /// Insert a new collection of static object keys, or return one already
    /// existing.
    pub(crate) fn new_static_object_keys<S, I>(
        &self,
        spanned: S,
        current: I,
    ) -> Result<usize, CompileError>
    where
        S: Copy + Spanned,
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut inner = self.inner.borrow_mut();

        let current = current
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect::<Box<_>>();

        let hash = Hash::object_keys(&current[..]);

        if let Some(existing_slot) = inner.static_object_keys_rev.get(&hash).copied() {
            let existing = inner.static_object_keys.get(existing_slot).ok_or_else(|| {
                CompileError::new(
                    spanned,
                    CompileErrorKind::StaticObjectKeysMissing {
                        hash,
                        slot: existing_slot,
                    },
                )
            })?;

            if *existing != current {
                return Err(CompileError::new(
                    spanned,
                    CompileErrorKind::StaticObjectKeysHashConflict {
                        hash,
                        current,
                        existing: existing.clone(),
                    },
                ));
            }

            return Ok(existing_slot);
        }

        let new_slot = inner.static_object_keys.len();
        inner.static_object_keys.push(current);
        inner.static_object_keys_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    /// Perform a path lookup on the current state of the unit.
    pub(crate) fn find_named(
        &self,
        base: &Item,
        mod_item: Option<&Rc<QueryMod>>,
        impl_item: Option<&Item>,
        path: &ast::Path,
        storage: &Storage,
        source: &Source,
    ) -> CompileResult<Named> {
        if let Some(global) = &path.global {
            return Err(CompileError::internal(
                global,
                "global scopes are not supported yet",
            ));
        }

        let inner = self.inner.borrow();
        let mut in_self_type = false;

        let (item, local) = match &path.first {
            ast::PathSegment::Ident(ident) => {
                let local = ident.resolve(storage, source)?;
                (base.clone(), Some(local))
            }
            ast::PathSegment::Super(super_value) => {
                let mut item = mod_item
                    .ok_or_else(CompileError::unsupported_super(super_value))?
                    .item
                    .clone();

                item.pop()
                    .ok_or_else(CompileError::unsupported_super(super_value))?;

                (item, None)
            }
            ast::PathSegment::SelfType(self_type) => {
                let impl_item = impl_item.ok_or_else(|| {
                    CompileError::new(self_type, CompileErrorKind::UnsupportedSelfType)
                })?;

                in_self_type = true;
                (impl_item.clone(), None)
            }
            ast::PathSegment::SelfValue(self_value) => {
                let mod_item = mod_item
                    .ok_or_else(|| {
                        CompileError::new(self_value, CompileErrorKind::UnsupportedSelfValue)
                    })?
                    .item
                    .clone();

                (mod_item.clone(), None)
            }
            ast::PathSegment::Crate(..) => (Item::new(), None),
        };

        let (mut item, imported) = if let Some(local) = local {
            match inner.lookup_import_by_name(&item, local.as_ref()) {
                Some(path) => (path, true),
                None => (Item::of(Some(local)), false),
            }
        } else {
            (item, false)
        };

        for (_, segment) in &path.rest {
            match segment {
                ast::PathSegment::Ident(ident) => {
                    item.push(ident.resolve(storage, source)?.as_ref());
                }
                ast::PathSegment::Super(super_token) => {
                    if in_self_type {
                        return Err(CompileError::new(
                            super_token,
                            CompileErrorKind::UnsupportedSuperInSelfType,
                        ));
                    }

                    item.pop()
                        .ok_or_else(CompileError::unsupported_super(super_token))?;
                }
                other => {
                    return Err(CompileError::new(
                        other,
                        CompileErrorKind::ExpectedLeadingPathSegment,
                    ));
                }
            }
        }

        Ok(Named {
            imported,
            local: path.global.is_none() && path.rest.is_empty(),
            item,
        })
    }

    /// Declare a new import.
    pub(crate) fn new_import<S, A>(
        &self,
        spanned: S,
        at: Item,
        path: Item,
        alias: Option<A>,
        source_id: usize,
    ) -> Result<(), CompileError>
    where
        S: Spanned,
        A: IntoComponent,
    {
        let mut inner = self.inner.borrow_mut();

        if let Some(last) = alias
            .as_ref()
            .map(IntoComponent::as_component_ref)
            .or_else(|| path.last())
        {
            let key = ImportKey::new(at, last.into_component());

            let entry = ImportEntry {
                item: path.clone(),
                span: Some((spanned.span(), source_id)),
            };

            if let Some(old) = inner.imports.insert(key.clone(), entry) {
                // NB: don't error if we're overwriting prelude.
                if let Some(existing) = old.span {
                    return Err(CompileError::new(
                        spanned,
                        CompileErrorKind::ImportConflict { key, existing },
                    ));
                }
            }
        }

        Ok(())
    }

    /// Insert the given name into the unit.
    pub(crate) fn insert_name(&self, item: &Item) {
        self.inner.borrow_mut().names.insert(item);
    }

    /// Declare a new struct.
    pub(crate) fn insert_meta(&self, at: Item, meta: CompileMeta) -> Result<(), InsertMetaError> {
        let mut inner = self.inner.borrow_mut();

        match &meta.kind {
            CompileMetaKind::UnitStruct { empty, .. } => {
                let info = UnitFn::UnitStruct { hash: empty.hash };

                let signature = DebugSignature {
                    path: empty.item.clone(),
                    args: DebugArgs::EmptyArgs,
                };

                let rtti = Arc::new(Rtti {
                    hash: empty.hash,
                    item: empty.item.clone(),
                });

                if inner.rtti.insert(empty.hash, rtti).is_some() {
                    return Err(InsertMetaError::TypeRttiConflict { hash: empty.hash });
                }

                if inner.functions.insert(empty.hash, info).is_some() {
                    return Err(InsertMetaError::FunctionConflict {
                        existing: signature,
                    });
                }

                let info = UnitTypeInfo {
                    hash: empty.hash,
                    type_of: Type::from(empty.hash),
                };

                if inner.types.insert(empty.hash, info).is_some() {
                    return Err(InsertMetaError::TypeConflict {
                        existing: empty.item.clone(),
                    });
                }

                inner
                    .debug_info_mut()
                    .functions
                    .insert(empty.hash, signature);
            }
            CompileMetaKind::TupleStruct { tuple, .. } => {
                let info = UnitFn::TupleStruct {
                    hash: tuple.hash,
                    args: tuple.args,
                };

                let signature = DebugSignature {
                    path: tuple.item.clone(),
                    args: DebugArgs::TupleArgs(tuple.args),
                };

                let rtti = Arc::new(Rtti {
                    hash: tuple.hash,
                    item: tuple.item.clone(),
                });

                if inner.rtti.insert(tuple.hash, rtti).is_some() {
                    return Err(InsertMetaError::TypeRttiConflict { hash: tuple.hash });
                }

                if inner.functions.insert(tuple.hash, info).is_some() {
                    return Err(InsertMetaError::FunctionConflict {
                        existing: signature,
                    });
                }

                let info = UnitTypeInfo {
                    hash: tuple.hash,
                    type_of: Type::from(tuple.hash),
                };

                if inner.types.insert(tuple.hash, info).is_some() {
                    return Err(InsertMetaError::TypeConflict {
                        existing: tuple.item.clone(),
                    });
                }

                inner
                    .debug_info_mut()
                    .functions
                    .insert(tuple.hash, signature);
            }
            CompileMetaKind::Struct { object, .. } => {
                let hash = Hash::type_hash(&object.item);

                let rtti = Arc::new(Rtti {
                    hash,
                    item: object.item.clone(),
                });

                if inner.rtti.insert(hash, rtti).is_some() {
                    return Err(InsertMetaError::TypeRttiConflict { hash });
                }

                let info = UnitTypeInfo {
                    hash,
                    type_of: Type::from(hash),
                };

                if inner.types.insert(hash, info).is_some() {
                    return Err(InsertMetaError::TypeConflict {
                        existing: object.item.clone(),
                    });
                }
            }
            CompileMetaKind::UnitVariant {
                enum_item, empty, ..
            } => {
                let enum_hash = Hash::type_hash(enum_item);

                let rtti = Arc::new(VariantRtti {
                    enum_hash,
                    hash: empty.hash,
                    item: empty.item.clone(),
                });

                if inner.variant_rtti.insert(empty.hash, rtti).is_some() {
                    return Err(InsertMetaError::VariantRttiConflict { hash: empty.hash });
                }

                let info = UnitFn::UnitVariant { hash: empty.hash };

                let signature = DebugSignature {
                    path: empty.item.clone(),
                    args: DebugArgs::EmptyArgs,
                };

                if inner.functions.insert(empty.hash, info).is_some() {
                    return Err(InsertMetaError::FunctionConflict {
                        existing: signature,
                    });
                }

                let info = UnitTypeInfo {
                    hash: empty.hash,
                    type_of: Type::from(enum_hash),
                };

                if inner.types.insert(empty.hash, info).is_some() {
                    return Err(InsertMetaError::TypeConflict {
                        existing: empty.item.clone(),
                    });
                }

                inner
                    .debug_info_mut()
                    .functions
                    .insert(empty.hash, signature);
            }
            CompileMetaKind::TupleVariant {
                enum_item, tuple, ..
            } => {
                let enum_hash = Hash::type_hash(enum_item);

                let rtti = Arc::new(VariantRtti {
                    enum_hash,
                    hash: tuple.hash,
                    item: tuple.item.clone(),
                });

                if inner.variant_rtti.insert(tuple.hash, rtti).is_some() {
                    return Err(InsertMetaError::VariantRttiConflict { hash: tuple.hash });
                }

                let info = UnitFn::TupleVariant {
                    hash: tuple.hash,
                    args: tuple.args,
                };

                let signature = DebugSignature {
                    path: tuple.item.clone(),
                    args: DebugArgs::TupleArgs(tuple.args),
                };

                if inner.functions.insert(tuple.hash, info).is_some() {
                    return Err(InsertMetaError::FunctionConflict {
                        existing: signature,
                    });
                }

                let info = UnitTypeInfo {
                    hash: tuple.hash,
                    type_of: Type::from(enum_hash),
                };

                if inner.types.insert(tuple.hash, info).is_some() {
                    return Err(InsertMetaError::TypeConflict {
                        existing: tuple.item.clone(),
                    });
                }

                inner
                    .debug_info_mut()
                    .functions
                    .insert(tuple.hash, signature);
            }
            CompileMetaKind::StructVariant {
                enum_item, object, ..
            } => {
                let hash = Hash::type_hash(&object.item);
                let enum_hash = Hash::type_hash(enum_item);

                let rtti = Arc::new(VariantRtti {
                    enum_hash,
                    hash,
                    item: object.item.clone(),
                });

                if inner.variant_rtti.insert(hash, rtti).is_some() {
                    return Err(InsertMetaError::VariantRttiConflict { hash });
                }

                let info = UnitTypeInfo {
                    hash,
                    type_of: Type::from(enum_hash),
                };

                if inner.types.insert(hash, info).is_some() {
                    return Err(InsertMetaError::TypeConflict {
                        existing: object.item.clone(),
                    });
                }
            }
            CompileMetaKind::Enum { item, .. } => {
                let hash = Hash::type_hash(&at);

                let info = UnitTypeInfo {
                    hash,
                    type_of: Type::from(hash),
                };

                if inner.types.insert(hash, info).is_some() {
                    return Err(InsertMetaError::TypeConflict {
                        existing: item.clone(),
                    });
                }
            }
            CompileMetaKind::Import { .. } => (),
            CompileMetaKind::Function { .. } => (),
            CompileMetaKind::Closure { .. } => (),
            CompileMetaKind::AsyncBlock { .. } => (),
            CompileMetaKind::Macro { .. } => (),
            CompileMetaKind::Const { .. } => (),
            CompileMetaKind::ConstFn { .. } => (),
        }

        if let Some(existing) = inner.meta.insert(at, meta.clone()) {
            return Err(InsertMetaError::MetaConflict {
                current: meta,
                existing,
            });
        }

        Ok(())
    }

    /// Construct a new empty assembly associated with the current unit.
    pub(crate) fn new_assembly<S>(&self, spanned: S, source_id: usize) -> Assembly
    where
        S: Spanned,
    {
        let label_count = self.inner.borrow().label_count;
        Assembly::new(source_id, spanned, label_count)
    }

    /// Declare a new function at the current instruction pointer.
    pub(crate) fn new_function<S>(
        &self,
        spanned: S,
        source_id: usize,
        path: Item,
        args: usize,
        assembly: Assembly,
        call: Call,
        debug_args: Vec<String>,
    ) -> Result<(), CompileError>
    where
        S: Spanned,
    {
        let mut inner = self.inner.borrow_mut();

        let offset = inner.instructions.len();
        let hash = Hash::type_hash(&path);

        inner.functions_rev.insert(offset, hash);
        let info = UnitFn::Offset { offset, call, args };
        let signature = DebugSignature::new(path, debug_args);

        if inner.functions.insert(hash, info).is_some() {
            return Err(CompileError::new(
                spanned,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        inner.debug_info_mut().functions.insert(hash, signature);
        inner.add_assembly(source_id, assembly)?;
        Ok(())
    }

    /// Declare a new instance function at the current instruction pointer.
    pub(crate) fn new_instance_function<S>(
        &self,
        spanned: S,
        source_id: usize,
        path: Item,
        type_of: Type,
        name: &str,
        args: usize,
        assembly: Assembly,
        call: Call,
        debug_args: Vec<String>,
    ) -> Result<(), CompileError>
    where
        S: Spanned,
    {
        log::trace!("instance fn: {}", path);

        let mut inner = self.inner.borrow_mut();

        let offset = inner.instructions.len();
        let instance_fn = Hash::instance_function(type_of, name);
        let hash = Hash::type_hash(&path);

        let info = UnitFn::Offset { offset, call, args };
        let signature = DebugSignature::new(path, debug_args);

        if inner.functions.insert(instance_fn, info).is_some() {
            return Err(CompileError::new(
                spanned,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        if inner.functions.insert(hash, info).is_some() {
            return Err(CompileError::new(
                spanned,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        inner
            .debug_info_mut()
            .functions
            .insert(instance_fn, signature);
        inner.functions_rev.insert(offset, hash);
        inner.add_assembly(source_id, assembly)?;
        Ok(())
    }

    /// Try to link the unit with the context, checking that all necessary
    /// functions are provided.
    ///
    /// This can prevent a number of runtime errors, like missing functions.
    pub(crate) fn link(&self, context: &Context, errors: &mut Errors) {
        let inner = self.inner.borrow();

        for (hash, spans) in &inner.required_functions {
            if inner.functions.get(hash).is_none() && context.lookup(*hash).is_none() {
                errors.push(Error::new(
                    0,
                    LinkerError::MissingFunction {
                        hash: *hash,
                        spans: spans.clone(),
                    },
                ));
            }
        }
    }
}

/// The result of calling [UnitBuilder::find_named].
#[derive(Debug)]
pub struct Named {
    /// If the named item was imported.
    pub imported: bool,
    /// If the resolved value is local.
    pub local: bool,
    /// The path resolved to the given item.
    pub item: Item,
}

impl Named {
    /// Get the local identifier of this named.
    pub fn as_local(&self) -> Option<&str> {
        if self.local {
            self.item.as_local()
        } else {
            None
        }
    }
}

impl fmt::Display for Named {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.item, f)
    }
}

/// An error raised during linking.
#[derive(Debug, Error)]
pub enum LinkerError {
    /// Missing a function with the given hash.
    #[error("missing function with hash {hash}")]
    MissingFunction {
        /// Hash of the function.
        hash: Hash,
        /// Spans where the function is used.
        spans: Vec<(Span, usize)>,
    },
}

#[derive(Debug, Default)]
struct Inner {
    /// The instructions contained in the source file.
    instructions: Vec<Inst>,
    /// All imports in the current unit.
    ///
    /// Only used to link against the current environment to make sure all
    /// required units are present.
    imports: HashMap<ImportKey, ImportEntry>,
    /// Item metadata in the context.
    meta: HashMap<Item, CompileMeta>,
    /// Where functions are located in the collection of instructions.
    functions: HashMap<Hash, UnitFn>,
    /// Declared types.
    types: HashMap<Hash, UnitTypeInfo>,
    /// Function by address.
    functions_rev: HashMap<usize, Hash>,
    /// A static string.
    static_strings: Vec<Arc<StaticString>>,
    /// Reverse lookup for static strings.
    static_string_rev: HashMap<Hash, usize>,
    /// A static byte string.
    static_bytes: Vec<Vec<u8>>,
    /// Reverse lookup for static byte strings.
    static_bytes_rev: HashMap<Hash, usize>,
    /// Slots used for object keys.
    ///
    /// This is used when an object is used in a pattern match, to avoid having
    /// to send the collection of keys to the virtual machine.
    ///
    /// All keys are sorted with the default string sort.
    static_object_keys: Vec<Box<[String]>>,
    /// Used to detect duplicates in the collection of static object keys.
    static_object_keys_rev: HashMap<Hash, usize>,
    /// Runtime type information for types.
    rtti: HashMap<Hash, Arc<Rtti>>,
    /// Runtime type information for variants.
    variant_rtti: HashMap<Hash, Arc<VariantRtti>>,
    /// The current label count.
    label_count: usize,
    /// A collection of required function hashes.
    required_functions: HashMap<Hash, Vec<(Span, usize)>>,
    /// All available names in the context.
    names: Names,
    /// Debug info if available for unit.
    debug: Option<Box<DebugInfo>>,
}

impl Inner {
    /// Insert and access debug information.
    fn debug_info_mut(&mut self) -> &mut DebugInfo {
        self.debug.get_or_insert_with(Default::default)
    }

    fn lookup_import_by_name(&self, base: &Item, local: &str) -> Option<Item> {
        let mut base = base.clone();

        loop {
            let key = ImportKey::new(base.clone(), local);

            if let Some(entry) = self.imports.get(&key) {
                return Some(entry.item.clone());
            }

            if base.pop().is_none() {
                break;
            }
        }

        None
    }

    /// Translate the given assembly into instructions.
    fn add_assembly(&mut self, source_id: usize, assembly: Assembly) -> Result<(), CompileError> {
        self.label_count = assembly.label_count;

        self.required_functions.extend(assembly.required_functions);

        for (pos, (inst, span)) in assembly.instructions.into_iter().enumerate() {
            let mut comment = None;
            let label = assembly.labels_rev.get(&pos).copied();

            match inst {
                AssemblyInst::Jump { label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::Jump { offset });
                }
                AssemblyInst::JumpIf { label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::JumpIf { offset });
                }
                AssemblyInst::JumpIfNot { label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::JumpIfNot { offset });
                }
                AssemblyInst::JumpIfOrPop { label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::JumpIfOrPop { offset });
                }
                AssemblyInst::JumpIfNotOrPop { label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::JumpIfNotOrPop { offset });
                }
                AssemblyInst::JumpIfBranch { branch, label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions
                        .push(Inst::JumpIfBranch { branch, offset });
                }
                AssemblyInst::PopAndJumpIfNot { count, label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions
                        .push(Inst::PopAndJumpIfNot { count, offset });
                }
                AssemblyInst::Raw { raw } => {
                    self.instructions.push(raw);
                }
            }

            if let Some(comments) = assembly.comments.get(&pos) {
                let actual = comment
                    .take()
                    .into_iter()
                    .chain(comments.iter().cloned())
                    .collect::<Vec<_>>()
                    .join("; ");
                comment = Some(actual)
            }

            let debug = self.debug.get_or_insert_with(Default::default);

            debug.instructions.push(DebugInst {
                source_id,
                span,
                comment,
                label: label.map(Label::into_owned),
            });
        }

        return Ok(());

        fn translate_offset(
            span: Span,
            base: usize,
            label: Label,
            labels: &HashMap<Label, usize>,
        ) -> Result<isize, CompileError> {
            use std::convert::TryFrom as _;

            let offset = labels
                .get(&label)
                .copied()
                .ok_or_else(|| CompileError::new(span, CompileErrorKind::MissingLabel { label }))?;

            let base = isize::try_from(base)
                .map_err(|_| CompileError::new(span, CompileErrorKind::BaseOverflow))?;
            let offset = isize::try_from(offset)
                .map_err(|_| CompileError::new(span, CompileErrorKind::OffsetOverflow))?;

            let (base, _) = base.overflowing_add(1);
            let (offset, _) = offset.overflowing_sub(base);
            Ok(offset)
        }
    }
}

/// Errors raised when building a new unit.
#[derive(Debug, Error)]
pub enum InsertMetaError {
    /// Trying to register a conflicting function.
    #[error("conflicting function signature already exists `{existing}`")]
    FunctionConflict {
        /// The signature of an already existing function.
        existing: DebugSignature,
    },
    /// Trying to insert a conflicting variant.
    #[error("tried to insert rtti for conflicting variant with hash `{hash}`")]
    VariantRttiConflict {
        /// The hash of the variant.
        hash: Hash,
    },
    /// Trying to insert a conflicting type.
    #[error("tried to insert rtti for conflicting type with hash `{hash}`")]
    TypeRttiConflict {
        /// The hash of the type.
        hash: Hash,
    },
    /// Tried to add an use that conflicts with an existing one.
    #[error("conflicting type already exists `{existing}`")]
    TypeConflict {
        /// The path to the existing type.
        existing: Item,
    },
    /// Tried to add an item that already exists.
    #[error("trying to insert `{current}` but conflicting meta `{existing}` already exists")]
    MetaConflict {
        /// The meta we tried to insert.
        current: CompileMeta,
        /// The existing item.
        existing: CompileMeta,
    },
}
