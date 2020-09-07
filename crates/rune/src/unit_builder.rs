//! A single execution unit in the runestick virtual machine.
//!
//! A unit consists of a sequence of instructions, and lookaside tables for
//! metadata like function locations.

use crate::assembly::{Assembly, AssemblyInst};
use crate::ast;
use crate::collections::HashMap;
use crate::error::CompileResult;
use crate::{Resolve as _, Storage};
use runestick::debug::{DebugArgs, DebugSignature};
use runestick::{
    Call, CompileMeta, Component, Context, DebugInfo, DebugInst, Hash, Inst, Item, Label, Names,
    Source, Span, StaticString, Type, Unit, UnitFn, UnitTypeInfo,
};
use std::sync::Arc;
use thiserror::Error;

/// Errors raised when building a new unit.
#[derive(Debug, Error)]
pub enum UnitBuilderError {
    /// Trying to register a conflicting function.
    #[error("conflicting function signature already exists `{existing}`")]
    FunctionConflict {
        /// The signature of an already existing function.
        existing: DebugSignature,
    },
    /// Tried to add an use that conflicts with an existing one.
    #[error("conflicting type already exists `{existing}`")]
    TypeConflict {
        /// The path to the existing type.
        existing: Item,
    },
    /// Tried to add an unsupported meta item to a unit.
    #[error("unsupported meta type for item `{existing}`")]
    UnsupportedMeta {
        /// The item used.
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
    /// A static string was missing for the given hash and slot.
    #[error("missing static string for hash `{hash}` and slot `{slot}`")]
    StaticStringMissing {
        /// The hash of the string.
        hash: Hash,
        /// The slot of the string.
        slot: usize,
    },
    /// A static byte string was missing for the given hash and slot.
    #[error("missing static byte string for hash `{hash}` and slot `{slot}`")]
    StaticBytesMissing {
        /// The hash of the byte string.
        hash: Hash,
        /// The slot of the byte string.
        slot: usize,
    },
    /// A static string was missing for the given hash and slot.
    #[error(
        "conflicting static string for hash `{hash}` between `{existing:?}` and `{current:?}`"
    )]
    StaticStringHashConflict {
        /// The hash of the string.
        hash: Hash,
        /// The static string that was inserted.
        current: String,
        /// The existing static string that conflicted.
        existing: String,
    },
    /// A static byte string was missing for the given hash and slot.
    #[error(
        "conflicting static string for hash `{hash}` between `{existing:?}` and `{current:?}`"
    )]
    StaticBytesHashConflict {
        /// The hash of the byte string.
        hash: Hash,
        /// The static byte string that was inserted.
        current: Vec<u8>,
        /// The existing static byte string that conflicted.
        existing: Vec<u8>,
    },
    /// A static object keys was missing for the given hash and slot.
    #[error("missing static object keys for hash `{hash}` and slot `{slot}`")]
    StaticObjectKeysMissing {
        /// The hash of the object keys.
        hash: Hash,
        /// The slot of the object keys.
        slot: usize,
    },
    /// A static object keys was missing for the given hash and slot.
    #[error(
        "conflicting static object keys for hash `{hash}` between `{existing:?}` and `{current:?}`"
    )]
    StaticObjectKeysHashConflict {
        /// The hash of the object keys.
        hash: Hash,
        /// The static object keys that was inserted.
        current: Box<[String]>,
        /// The existing static object keys that conflicted.
        existing: Box<[String]>,
    },
    /// Tried to add a duplicate label.
    #[error("duplicate label `{label}`")]
    DuplicateLabel {
        /// The duplicate label.
        label: Label,
    },
    /// The specified label is missing.
    #[error("missing label `{label}`")]
    MissingLabel {
        /// The missing label.
        label: Label,
    },
    /// Overflow error.
    #[error("base offset overflow")]
    BaseOverflow,
    /// Overflow error.
    #[error("offset overflow")]
    OffsetOverflow,
}

/// The key of an import.
#[derive(Debug, PartialEq, Eq, Hash)]
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
        C: Into<Component>,
    {
        Self {
            item,
            component: component.into(),
        }
    }

    /// Construct an import key for a single component.
    pub fn component<C>(component: C) -> Self
    where
        C: Into<Component>,
    {
        Self {
            item: Item::empty(),
            component: component.into(),
        }
    }
}

/// An imported entry.
#[derive(Debug)]
pub struct ImportEntry {
    /// The item being imported.
    pub item: Item,
    /// The span of the import.
    pub span: Option<(Span, usize)>,
}

impl ImportEntry {
    /// Construct an entry.
    pub fn of<I>(iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Component>,
    {
        Self {
            item: Item::of(iter),
            span: None,
        }
    }
}

/// Instructions from a single source file.
#[derive(Debug, Default)]
pub struct UnitBuilder {
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
    /// The current label count.
    label_count: usize,
    /// A collection of required function hashes.
    required_functions: HashMap<Hash, Vec<(Span, usize)>>,
    /// All available names in the context.
    names: Names,
    /// Debug info if available for unit.
    debug: Option<Box<DebugInfo>>,
}

impl UnitBuilder {
    /// Construct a new unit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a new unit with the default prelude.
    pub fn with_default_prelude() -> Self {
        let mut this = Self::new();

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

        this
    }

    /// Convert into a runtime unit, shedding our build metadata in the process.
    pub fn into_unit(mut self) -> Unit {
        if let Some(debug) = &mut self.debug {
            debug.functions_rev = self.functions_rev;
        }

        Unit::new(
            self.instructions,
            self.functions,
            self.types,
            self.static_strings,
            self.static_bytes,
            self.static_object_keys,
            self.debug,
        )
    }

    /// Insert and access debug information.
    pub(crate) fn debug_info_mut(&mut self) -> &mut DebugInfo {
        self.debug.get_or_insert_with(Default::default)
    }

    /// Check if unit contains the given name by prefix.
    pub(crate) fn contains_prefix(&self, item: &Item) -> bool {
        self.names.contains_prefix(item)
    }

    /// Iterate over registered imports.
    pub(crate) fn iter_imports<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&'a ImportKey, &'a ImportEntry)> + '_ {
        self.imports.iter()
    }

    /// Iterate over known child components of the given name.
    pub(crate) fn iter_components<I>(&self, iter: I) -> impl Iterator<Item = &'_ Component>
    where
        I: IntoIterator,
        I::Item: Into<Component>,
    {
        self.names.iter_components(iter)
    }

    /// Access the meta for the given language item.
    pub(crate) fn lookup_meta(&self, name: &Item) -> Option<CompileMeta> {
        self.meta.get(name).cloned()
    }

    /// Insert a static string and return its associated slot that can later be
    /// looked up through [lookup_string][Self::lookup_string].
    ///
    /// Only uses up space if the static string is unique.
    pub(crate) fn new_static_string(&mut self, current: &str) -> Result<usize, UnitBuilderError> {
        let current = StaticString::new(current);
        let hash = current.hash();

        if let Some(existing_slot) = self.static_string_rev.get(&hash).copied() {
            let existing = self.static_strings.get(existing_slot).ok_or_else(|| {
                UnitBuilderError::StaticStringMissing {
                    hash,
                    slot: existing_slot,
                }
            })?;

            if ***existing != *current {
                return Err(UnitBuilderError::StaticStringHashConflict {
                    hash,
                    current: (*current).clone(),
                    existing: (***existing).clone(),
                });
            }

            return Ok(existing_slot);
        }

        let new_slot = self.static_strings.len();
        self.static_strings.push(Arc::new(current));
        self.static_string_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    /// Insert a static byte string and return its associated slot that can
    /// later be looked up through [lookup_bytes][Self::lookup_bytes].
    ///
    /// Only uses up space if the static byte string is unique.
    pub(crate) fn new_static_bytes(&mut self, current: &[u8]) -> Result<usize, UnitBuilderError> {
        let hash = Hash::of(&current);

        if let Some(existing_slot) = self.static_bytes_rev.get(&hash).copied() {
            let existing = self.static_bytes.get(existing_slot).ok_or_else(|| {
                UnitBuilderError::StaticBytesMissing {
                    hash,
                    slot: existing_slot,
                }
            })?;

            if &**existing != current {
                return Err(UnitBuilderError::StaticBytesHashConflict {
                    hash,
                    current: current.to_owned(),
                    existing: existing.clone(),
                });
            }

            return Ok(existing_slot);
        }

        let new_slot = self.static_bytes.len();
        self.static_bytes.push(current.to_owned());
        self.static_bytes_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    /// Insert a new collection of static object keys, or return one already
    /// existing.
    pub(crate) fn new_static_object_keys(
        &mut self,
        current: &[String],
    ) -> Result<usize, UnitBuilderError> {
        let current = current.to_vec().into_boxed_slice();
        let hash = Hash::object_keys(&current[..]);

        if let Some(existing_slot) = self.static_object_keys_rev.get(&hash).copied() {
            let existing = self.static_object_keys.get(existing_slot).ok_or_else(|| {
                UnitBuilderError::StaticObjectKeysMissing {
                    hash,
                    slot: existing_slot,
                }
            })?;

            if *existing != current {
                return Err(UnitBuilderError::StaticObjectKeysHashConflict {
                    hash,
                    current,
                    existing: existing.clone(),
                });
            }

            return Ok(existing_slot);
        }

        let new_slot = self.static_object_keys.len();
        self.static_object_keys.push(current);
        self.static_object_keys_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    fn lookup_import_by_name(&self, base: &Item, local: &Component) -> Option<Item> {
        let mut base = base.clone();

        loop {
            let key = ImportKey::new(base.clone(), local.clone());

            if let Some(entry) = self.lookup_import(&key) {
                return Some(entry.item.clone());
            }

            if base.pop().is_none() {
                break;
            }
        }

        None
    }

    /// Perform a path lookup on the current state of the unit.
    pub(crate) fn convert_path(
        &self,
        base: &Item,
        path: &ast::Path,
        storage: &Storage,
        source: &Source,
    ) -> CompileResult<Item> {
        let local = Component::from(path.first.resolve(storage, source)?.as_ref());

        let imported = match self.lookup_import_by_name(base, &local) {
            Some(path) => path,
            None => Item::of(&[local]),
        };

        let mut rest = Vec::new();

        for (_, part) in &path.rest {
            rest.push(Component::String(
                part.resolve(storage, source)?.to_string(),
            ));
        }

        let it = imported.into_iter().chain(rest.into_iter());
        Ok(Item::of(it))
    }

    /// Look up an use by name.
    pub(crate) fn lookup_import(&self, key: &ImportKey) -> Option<&ImportEntry> {
        self.imports.get(&key)
    }

    /// Declare a new import.
    pub(crate) fn new_import<I>(
        &mut self,
        item: Item,
        path: I,
        span: Span,
        source_id: usize,
    ) -> Result<(), UnitBuilderError>
    where
        I: Copy + IntoIterator,
        I::Item: Into<Component>,
    {
        let path = Item::of(path);

        if let Some(last) = path.last() {
            let entry = ImportEntry {
                item: path.clone(),
                span: Some((span, source_id)),
            };

            self.imports
                .insert(ImportKey::new(item, last.clone()), entry);
        }

        Ok(())
    }

    /// Insert the given name into the unit.
    pub(crate) fn insert_name(&mut self, item: &Item) {
        self.names.insert(item);
    }

    /// Declare a new struct.
    pub(crate) fn insert_meta(&mut self, meta: CompileMeta) -> Result<(), UnitBuilderError> {
        let item = match &meta {
            CompileMeta::Tuple { tuple, .. } => {
                let info = UnitFn::Tuple {
                    hash: tuple.hash,
                    args: tuple.args,
                };

                let signature = DebugSignature {
                    path: tuple.item.clone(),
                    args: DebugArgs::TupleArgs(tuple.args),
                };

                if self.functions.insert(tuple.hash, info).is_some() {
                    return Err(UnitBuilderError::FunctionConflict {
                        existing: signature,
                    });
                }

                let info = UnitTypeInfo {
                    hash: tuple.hash,
                    value_type: Type::Hash(tuple.hash),
                };

                if self.types.insert(tuple.hash, info).is_some() {
                    return Err(UnitBuilderError::TypeConflict {
                        existing: tuple.item.clone(),
                    });
                }

                self.debug_info_mut()
                    .functions
                    .insert(tuple.hash, signature);

                tuple.item.clone()
            }
            CompileMeta::TupleVariant {
                enum_item, tuple, ..
            } => {
                let enum_hash = Hash::type_hash(enum_item);

                let info = UnitFn::TupleVariant {
                    enum_hash,
                    hash: tuple.hash,
                    args: tuple.args,
                };

                let signature = DebugSignature {
                    path: tuple.item.clone(),
                    args: DebugArgs::TupleArgs(tuple.args),
                };

                if self.functions.insert(tuple.hash, info).is_some() {
                    return Err(UnitBuilderError::FunctionConflict {
                        existing: signature,
                    });
                }

                let info = UnitTypeInfo {
                    hash: tuple.hash,
                    value_type: Type::Hash(enum_hash),
                };

                if self.types.insert(tuple.hash, info).is_some() {
                    return Err(UnitBuilderError::TypeConflict {
                        existing: tuple.item.clone(),
                    });
                }

                self.debug_info_mut()
                    .functions
                    .insert(tuple.hash, signature);

                tuple.item.clone()
            }
            CompileMeta::Struct { object, .. } => {
                let hash = Hash::type_hash(&object.item);

                let info = UnitTypeInfo {
                    hash,
                    value_type: Type::Hash(hash),
                };

                if self.types.insert(hash, info).is_some() {
                    return Err(UnitBuilderError::TypeConflict {
                        existing: object.item.clone(),
                    });
                }

                object.item.clone()
            }
            CompileMeta::StructVariant {
                enum_item, object, ..
            } => {
                let hash = Hash::type_hash(&object.item);
                let enum_hash = Hash::type_hash(enum_item);

                let info = UnitTypeInfo {
                    hash,
                    value_type: Type::Hash(enum_hash),
                };

                if self.types.insert(hash, info).is_some() {
                    return Err(UnitBuilderError::TypeConflict {
                        existing: object.item.clone(),
                    });
                }

                object.item.clone()
            }
            CompileMeta::Enum { item, .. } => {
                let hash = Hash::type_hash(item);

                let info = UnitTypeInfo {
                    hash,
                    value_type: Type::Hash(hash),
                };

                if self.types.insert(hash, info).is_some() {
                    return Err(UnitBuilderError::TypeConflict {
                        existing: item.clone(),
                    });
                }

                item.clone()
            }
            CompileMeta::Function { item, .. } => item.clone(),
            CompileMeta::Closure { item, .. } => item.clone(),
            CompileMeta::AsyncBlock { item, .. } => item.clone(),
            CompileMeta::Macro { item, .. } => item.clone(),
        };

        if let Some(existing) = self.meta.insert(item, meta.clone()) {
            return Err(UnitBuilderError::MetaConflict {
                current: meta,
                existing,
            });
        }

        Ok(())
    }

    /// Construct a new empty assembly associated with the current unit.
    pub(crate) fn new_assembly(&self, source_id: usize) -> Assembly {
        Assembly::new(source_id, self.label_count)
    }

    /// Declare a new function at the current instruction pointer.
    pub(crate) fn new_function(
        &mut self,
        source_id: usize,
        path: Item,
        args: usize,
        assembly: Assembly,
        call: Call,
        debug_args: Vec<String>,
    ) -> Result<(), UnitBuilderError> {
        let offset = self.instructions.len();
        let hash = Hash::type_hash(&path);

        self.functions_rev.insert(offset, hash);
        let info = UnitFn::Offset { offset, call, args };
        let signature = DebugSignature::new(path, debug_args);

        if self.functions.insert(hash, info).is_some() {
            return Err(UnitBuilderError::FunctionConflict {
                existing: signature,
            });
        }

        self.debug_info_mut().functions.insert(hash, signature);
        self.add_assembly(source_id, assembly)?;
        Ok(())
    }

    /// Declare a new instance function at the current instruction pointer.
    pub(crate) fn new_instance_function(
        &mut self,
        source_id: usize,
        path: Item,
        value_type: Type,
        name: &str,
        args: usize,
        assembly: Assembly,
        call: Call,
        debug_args: Vec<String>,
    ) -> Result<(), UnitBuilderError> {
        log::trace!("instance fn: {}", path);

        let offset = self.instructions.len();
        let instance_fn = Hash::of(name);
        let instance_fn = Hash::instance_function(value_type, instance_fn);
        let hash = Hash::type_hash(&path);

        let info = UnitFn::Offset { offset, call, args };
        let signature = DebugSignature::new(path, debug_args);

        if self.functions.insert(instance_fn, info.clone()).is_some() {
            return Err(UnitBuilderError::FunctionConflict {
                existing: signature,
            });
        }

        if self.functions.insert(hash, info).is_some() {
            return Err(UnitBuilderError::FunctionConflict {
                existing: signature,
            });
        }

        self.debug_info_mut()
            .functions
            .insert(instance_fn, signature);
        self.functions_rev.insert(offset, hash);
        self.add_assembly(source_id, assembly)?;
        Ok(())
    }

    /// Translate the given assembly into instructions.
    fn add_assembly(
        &mut self,
        source_id: usize,
        assembly: Assembly,
    ) -> Result<(), UnitBuilderError> {
        self.label_count = assembly.label_count;

        self.required_functions.extend(assembly.required_functions);

        for (pos, (inst, span)) in assembly.instructions.into_iter().enumerate() {
            let mut comment = None;
            let label = assembly.labels_rev.get(&pos).copied();

            match inst {
                AssemblyInst::Jump { label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::Jump { offset });
                }
                AssemblyInst::JumpIf { label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::JumpIf { offset });
                }
                AssemblyInst::JumpIfNot { label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::JumpIfNot { offset });
                }
                AssemblyInst::JumpIfBranch { branch, label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(pos, label, &assembly.labels)?;
                    self.instructions
                        .push(Inst::JumpIfBranch { branch, offset });
                }
                AssemblyInst::PopAndJumpIfNot { count, label } => {
                    comment = Some(format!("label:{}", label));
                    let offset = translate_offset(pos, label, &assembly.labels)?;
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
                label,
            });
        }

        return Ok(());

        fn translate_offset(
            base: usize,
            label: Label,
            labels: &HashMap<Label, usize>,
        ) -> Result<isize, UnitBuilderError> {
            use std::convert::TryFrom as _;

            let offset = labels
                .get(&label)
                .copied()
                .ok_or_else(|| UnitBuilderError::MissingLabel { label })?;

            let base = isize::try_from(base).map_err(|_| UnitBuilderError::BaseOverflow)?;
            let offset = isize::try_from(offset).map_err(|_| UnitBuilderError::OffsetOverflow)?;

            let (base, _) = base.overflowing_add(1);
            let (offset, _) = offset.overflowing_sub(base);
            Ok(offset)
        }
    }

    /// Try to link the unit with the context, checking that all necessary
    /// functions are provided.
    ///
    /// This can prevent a number of runtime errors, like missing functions.
    pub(crate) fn link(&self, context: &Context, errors: &mut LinkerErrors) -> bool {
        for (hash, spans) in &self.required_functions {
            if self.functions.get(hash).is_none() && context.lookup(*hash).is_none() {
                errors.errors.push(LinkerError::MissingFunction {
                    hash: *hash,
                    spans: spans.clone(),
                });
            }
        }

        errors.errors.is_empty()
    }
}

/// An error raised during linking.
#[derive(Debug)]
pub enum LinkerError {
    /// Missing a function with the given hash.
    MissingFunction {
        /// Hash of the function.
        hash: Hash,
        /// Spans where the function is used.
        spans: Vec<(Span, usize)>,
    },
}

/// Linker errors.
#[derive(Debug, Default)]
pub struct LinkerErrors {
    errors: Vec<LinkerError>,
}

impl LinkerErrors {
    /// Construct a new collection of linker errors.
    pub fn new() -> Self {
        Self::default()
    }

    /// Test if error collection is empty.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Return an iterator over all linker errors.
    pub fn errors(self) -> impl Iterator<Item = LinkerError> {
        self.errors.into_iter()
    }
}

impl<'a> IntoIterator for &'a LinkerErrors {
    type IntoIter = std::slice::Iter<'a, LinkerError>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.iter()
    }
}
