//! A single execution unit in the runestick virtual machine.
//!
//! A unit consists of a sequence of instructions, and lookaside tables for
//! metadata like function locations.

use crate::ast::Span;
use crate::collections::HashMap;
use crate::compile::{
    Assembly, AssemblyInst, CompileError, CompileErrorKind, IntoComponent, Item, Location, Meta,
    MetaKind,
};
use crate::query::{QueryError, QueryErrorKind};
use crate::runtime::debug::{DebugArgs, DebugSignature};
use crate::runtime::{
    Call, ConstValue, DebugInfo, DebugInst, Inst, Label, Rtti, StaticString, Unit, UnitFn,
    VariantRtti,
};
use crate::{Context, Diagnostics, Hash, Protocol, SourceId};
use std::sync::Arc;
use thiserror::Error;

/// Errors that can be raised when linking units.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum LinkerError {
    #[error("missing function with hash {hash}")]
    MissingFunction {
        hash: Hash,
        spans: Vec<(Span, SourceId)>,
    },
}

/// Instructions from a single source file.
#[derive(Debug, Default)]
pub(crate) struct UnitBuilder {
    /// Prelude imports.
    prelude: HashMap<Box<str>, Item>,
    /// The instructions contained in the source file.
    instructions: Vec<Inst>,
    /// Registered re-exports.
    reexports: HashMap<Hash, Hash>,
    /// Where functions are located in the collection of instructions.
    functions: HashMap<Hash, UnitFn>,
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
    required_functions: HashMap<Hash, Vec<(Span, SourceId)>>,
    /// Debug info if available for unit.
    debug: Option<Box<DebugInfo>>,
    /// Constant values
    constants: HashMap<Hash, ConstValue>,
}

impl UnitBuilder {
    /// Construct a new unit with the default prelude.
    pub(crate) fn with_default_prelude() -> Self {
        let mut this = Self::default();

        this.add_prelude("assert_eq", &["test", "assert_eq"]);
        this.add_prelude("assert", &["test", "assert"]);
        this.add_prelude("bool", &["bool"]);
        this.add_prelude("byte", &["byte"]);
        this.add_prelude("char", &["char"]);
        this.add_prelude("dbg", &["io", "dbg"]);
        this.add_prelude("drop", &["mem", "drop"]);
        this.add_prelude("Err", &["result", "Result", "Err"]);
        this.add_prelude("file", &["macros", "builtin", "file"]);
        this.add_prelude("float", &["float"]);
        this.add_prelude("format", &["fmt", "format"]);
        this.add_prelude("int", &["int"]);
        this.add_prelude("is_readable", &["is_readable"]);
        this.add_prelude("is_writable", &["is_writable"]);
        this.add_prelude("line", &["macros", "builtin", "line"]);
        this.add_prelude("None", &["option", "Option", "None"]);
        this.add_prelude("Object", &["object", "Object"]);
        this.add_prelude("Ok", &["result", "Result", "Ok"]);
        this.add_prelude("Option", &["option", "Option"]);
        this.add_prelude("panic", &["panic"]);
        this.add_prelude("print", &["io", "print"]);
        this.add_prelude("println", &["io", "println"]);
        this.add_prelude("Result", &["result", "Result"]);
        this.add_prelude("Some", &["option", "Option", "Some"]);
        this.add_prelude("String", &["string", "String"]);
        this.add_prelude("stringify", &["stringify"]);
        this.add_prelude("unit", &["unit"]);
        this.add_prelude("Vec", &["vec", "Vec"]);

        this
    }

    /// Clone the prelude.
    pub(crate) fn prelude(&self) -> &HashMap<Box<str>, Item> {
        &self.prelude
    }

    /// Convert into a runtime unit, shedding our build metadata in the process.
    ///
    /// Returns `None` if the builder is still in use.
    pub(crate) fn build(mut self, span: Span) -> Result<Unit, CompileError> {
        if let Some(debug) = &mut self.debug {
            debug.functions_rev = self.functions_rev;
        }

        for (from, to) in self.reexports {
            let info = match self.functions.get(&to) {
                Some(info) => *info,
                None => {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::MissingFunctionHash { hash: to },
                    ))
                }
            };

            if self.functions.insert(from, info).is_some() {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::FunctionConflictHash { hash: from },
                ));
            }
        }

        Ok(Unit::new(
            self.instructions,
            self.functions,
            self.static_strings,
            self.static_bytes,
            self.static_object_keys,
            self.rtti,
            self.variant_rtti,
            self.debug,
            self.constants,
        ))
    }

    /// Insert a static string and return its associated slot that can later be
    /// looked up through [lookup_string][Unit::lookup_string].
    ///
    /// Only uses up space if the static string is unique.
    pub(crate) fn new_static_string(
        &mut self,
        span: Span,
        current: &str,
    ) -> Result<usize, CompileError> {
        let current = StaticString::new(current);
        let hash = current.hash();

        if let Some(existing_slot) = self.static_string_rev.get(&hash).copied() {
            let existing = self.static_strings.get(existing_slot).ok_or_else(|| {
                CompileError::new(
                    span,
                    CompileErrorKind::StaticStringMissing {
                        hash,
                        slot: existing_slot,
                    },
                )
            })?;

            if ***existing != *current {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::StaticStringHashConflict {
                        hash,
                        current: (*current).clone(),
                        existing: (***existing).clone(),
                    },
                ));
            }

            return Ok(existing_slot);
        }

        let new_slot = self.static_strings.len();
        self.static_strings.push(Arc::new(current));
        self.static_string_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    /// Insert a static byte string and return its associated slot that can
    /// later be looked up through [lookup_bytes][Unit::lookup_bytes].
    ///
    /// Only uses up space if the static byte string is unique.
    pub(crate) fn new_static_bytes(
        &mut self,
        span: Span,
        current: &[u8],
    ) -> Result<usize, CompileError> {
        let hash = Hash::static_bytes(current);

        if let Some(existing_slot) = self.static_bytes_rev.get(&hash).copied() {
            let existing = self.static_bytes.get(existing_slot).ok_or_else(|| {
                CompileError::new(
                    span,
                    CompileErrorKind::StaticBytesMissing {
                        hash,
                        slot: existing_slot,
                    },
                )
            })?;

            if &**existing != current {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::StaticBytesHashConflict {
                        hash,
                        current: current.to_owned(),
                        existing: existing.clone(),
                    },
                ));
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
    pub(crate) fn new_static_object_keys_iter<I>(
        &mut self,
        span: Span,
        current: I,
    ) -> Result<usize, CompileError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let current = current
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect::<Box<_>>();

        self.new_static_object_keys(span, current)
    }

    /// Insert a new collection of static object keys, or return one already
    /// existing.
    pub(crate) fn new_static_object_keys(
        &mut self,
        span: Span,
        current: Box<[String]>,
    ) -> Result<usize, CompileError> {
        let hash = Hash::object_keys(&current[..]);

        if let Some(existing_slot) = self.static_object_keys_rev.get(&hash).copied() {
            let existing = self.static_object_keys.get(existing_slot).ok_or_else(|| {
                CompileError::new(
                    span,
                    CompileErrorKind::StaticObjectKeysMissing {
                        hash,
                        slot: existing_slot,
                    },
                )
            })?;

            if *existing != current {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::StaticObjectKeysHashConflict {
                        hash,
                        current,
                        existing: existing.clone(),
                    },
                ));
            }

            return Ok(existing_slot);
        }

        let new_slot = self.static_object_keys.len();
        self.static_object_keys.push(current);
        self.static_object_keys_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    /// Declare a new struct.
    pub(crate) fn insert_meta(&mut self, span: Span, meta: &Meta) -> Result<(), QueryError> {
        match &meta.kind {
            MetaKind::UnitStruct { empty, .. } => {
                let info = UnitFn::UnitStruct { hash: empty.hash };

                let signature = DebugSignature::new(meta.item.item.clone(), DebugArgs::EmptyArgs);

                let rtti = Arc::new(Rtti {
                    hash: empty.hash,
                    item: meta.item.item.clone(),
                });

                if self.rtti.insert(empty.hash, rtti).is_some() {
                    return Err(QueryError::new(
                        span,
                        QueryErrorKind::TypeRttiConflict { hash: empty.hash },
                    ));
                }

                if self.functions.insert(empty.hash, info).is_some() {
                    return Err(QueryError::new(
                        span,
                        QueryErrorKind::FunctionConflict {
                            existing: signature,
                        },
                    ));
                }

                self.constants.insert(
                    Hash::instance_function(empty.hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(signature.path.to_string()),
                );

                self.debug_info_mut()
                    .functions
                    .insert(empty.hash, signature);
            }
            MetaKind::TupleStruct { tuple, .. } => {
                let info = UnitFn::TupleStruct {
                    hash: tuple.hash,
                    args: tuple.args,
                };

                let signature =
                    DebugSignature::new(meta.item.item.clone(), DebugArgs::TupleArgs(tuple.args));

                let rtti = Arc::new(Rtti {
                    hash: tuple.hash,
                    item: meta.item.item.clone(),
                });

                if self.rtti.insert(tuple.hash, rtti).is_some() {
                    return Err(QueryError::new(
                        span,
                        QueryErrorKind::TypeRttiConflict { hash: tuple.hash },
                    ));
                }

                if self.functions.insert(tuple.hash, info).is_some() {
                    return Err(QueryError::new(
                        span,
                        QueryErrorKind::FunctionConflict {
                            existing: signature,
                        },
                    ));
                }

                self.constants.insert(
                    Hash::instance_function(tuple.hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(signature.path.to_string()),
                );

                self.debug_info_mut()
                    .functions
                    .insert(tuple.hash, signature);
            }
            MetaKind::Struct { .. } => {
                let hash = Hash::type_hash(&meta.item.item);

                let rtti = Arc::new(Rtti {
                    hash,
                    item: meta.item.item.clone(),
                });

                self.constants.insert(
                    Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(rtti.item.to_string()),
                );

                if self.rtti.insert(hash, rtti).is_some() {
                    return Err(QueryError::new(
                        span,
                        QueryErrorKind::TypeRttiConflict { hash },
                    ));
                }
            }
            MetaKind::UnitVariant {
                enum_item, empty, ..
            } => {
                let enum_hash = Hash::type_hash(enum_item);

                let rtti = Arc::new(VariantRtti {
                    enum_hash,
                    hash: empty.hash,
                    item: meta.item.item.clone(),
                });

                if self.variant_rtti.insert(empty.hash, rtti).is_some() {
                    return Err(QueryError::new(
                        span,
                        QueryErrorKind::VariantRttiConflict { hash: empty.hash },
                    ));
                }

                let info = UnitFn::UnitVariant { hash: empty.hash };

                let signature = DebugSignature::new(meta.item.item.clone(), DebugArgs::EmptyArgs);

                if self.functions.insert(empty.hash, info).is_some() {
                    return Err(QueryError::new(
                        span,
                        QueryErrorKind::FunctionConflict {
                            existing: signature,
                        },
                    ));
                }

                self.debug_info_mut()
                    .functions
                    .insert(empty.hash, signature);
            }
            MetaKind::TupleVariant {
                enum_item, tuple, ..
            } => {
                let enum_hash = Hash::type_hash(enum_item);

                let rtti = Arc::new(VariantRtti {
                    enum_hash,
                    hash: tuple.hash,
                    item: meta.item.item.clone(),
                });

                if self.variant_rtti.insert(tuple.hash, rtti).is_some() {
                    return Err(QueryError::new(
                        span,
                        QueryErrorKind::VariantRttiConflict { hash: tuple.hash },
                    ));
                }

                let info = UnitFn::TupleVariant {
                    hash: tuple.hash,
                    args: tuple.args,
                };

                let signature =
                    DebugSignature::new(meta.item.item.clone(), DebugArgs::TupleArgs(tuple.args));

                if self.functions.insert(tuple.hash, info).is_some() {
                    return Err(QueryError::new(
                        span,
                        QueryErrorKind::FunctionConflict {
                            existing: signature,
                        },
                    ));
                }

                self.debug_info_mut()
                    .functions
                    .insert(tuple.hash, signature);
            }
            MetaKind::StructVariant { enum_item, .. } => {
                let hash = Hash::type_hash(&meta.item.item);
                let enum_hash = Hash::type_hash(enum_item);

                let rtti = Arc::new(VariantRtti {
                    enum_hash,
                    hash,
                    item: meta.item.item.clone(),
                });

                if self.variant_rtti.insert(hash, rtti).is_some() {
                    return Err(QueryError::new(
                        span,
                        QueryErrorKind::VariantRttiConflict { hash },
                    ));
                }
            }
            MetaKind::Enum { type_hash } => {
                self.constants.insert(
                    Hash::instance_function(*type_hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(meta.item.item.to_string()),
                );
            }

            MetaKind::Function { .. } => (),
            MetaKind::Closure { .. } => (),
            MetaKind::AsyncBlock { .. } => (),
            MetaKind::Const { const_value } => {
                self.constants
                    .insert(Hash::constant(&meta.item.item.to_string()), const_value.clone());
            }
            MetaKind::ConstFn { .. } => (),
            MetaKind::Import { .. } => (),
        }

        Ok(())
    }

    /// Construct a new empty assembly associated with the current unit.
    pub(crate) fn new_assembly(&self, location: Location) -> Assembly {
        Assembly::new(location, self.label_count)
    }

    /// Declare a new function at the current instruction pointer.
    pub(crate) fn new_function(
        &mut self,
        location: Location,
        path: Item,
        args: usize,
        assembly: Assembly,
        call: Call,
        debug_args: Box<[Box<str>]>,
    ) -> Result<(), CompileError> {
        let offset = self.instructions.len();
        let hash = Hash::type_hash(&path);

        self.functions_rev.insert(offset, hash);
        let info = UnitFn::Offset { offset, call, args };
        let signature = DebugSignature::new(path, DebugArgs::Named(debug_args));

        if self.functions.insert(hash, info).is_some() {
            return Err(CompileError::new(
                location.span,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        self.constants.insert(
            Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(signature.path.to_string()),
        );

        self.debug_info_mut().functions.insert(hash, signature);

        self.add_assembly(location, assembly)?;
        Ok(())
    }

    /// Register a new function re-export.
    pub(crate) fn new_function_reexport(
        &mut self,
        location: Location,
        item: &Item,
        target: &Item,
    ) -> Result<(), CompileError> {
        let hash = Hash::type_hash(item);
        let target = Hash::type_hash(target);

        if self.reexports.insert(hash, target).is_some() {
            return Err(CompileError::new(
                location.span,
                CompileErrorKind::FunctionReExportConflict { hash },
            ));
        }

        Ok(())
    }

    /// Declare a new instance function at the current instruction pointer.
    pub(crate) fn new_instance_function(
        &mut self,
        location: Location,
        path: Item,
        type_hash: Hash,
        name: &str,
        args: usize,
        assembly: Assembly,
        call: Call,
        debug_args: Box<[Box<str>]>,
    ) -> Result<(), CompileError> {
        log::trace!("instance fn: {}", path);

        let offset = self.instructions.len();
        let instance_fn = Hash::instance_function(type_hash, name);
        let hash = Hash::type_hash(&path);

        let info = UnitFn::Offset { offset, call, args };
        let signature = DebugSignature::new(path, DebugArgs::Named(debug_args));

        if self.functions.insert(instance_fn, info).is_some() {
            return Err(CompileError::new(
                location.span,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        if self.functions.insert(hash, info).is_some() {
            return Err(CompileError::new(
                location.span,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        self.constants.insert(
            Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(signature.path.to_string()),
        );

        self.debug_info_mut()
            .functions
            .insert(instance_fn, signature);
        self.functions_rev.insert(offset, hash);
        self.add_assembly(location, assembly)?;
        Ok(())
    }

    /// Try to link the unit with the context, checking that all necessary
    /// functions are provided.
    ///
    /// This can prevent a number of runtime errors, like missing functions.
    pub(crate) fn link(&mut self, context: &Context, diagnostics: &mut Diagnostics) {
        for (hash, spans) in &self.required_functions {
            if self.functions.get(hash).is_none() && context.lookup_function(*hash).is_none() {
                diagnostics.error(
                    SourceId::empty(),
                    LinkerError::MissingFunction {
                        hash: *hash,
                        spans: spans.clone(),
                    },
                );
            }
        }
    }

    /// Define a prelude item.
    fn add_prelude<I>(&mut self, local: &str, path: I)
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        self.prelude
            .insert(local.into(), Item::with_crate_item("std", path));
    }

    /// Insert and access debug information.
    fn debug_info_mut(&mut self) -> &mut DebugInfo {
        self.debug.get_or_insert_with(Default::default)
    }

    /// Translate the given assembly into instructions.
    fn add_assembly(&mut self, location: Location, assembly: Assembly) -> Result<(), CompileError> {
        self.label_count = assembly.label_count;

        self.required_functions.extend(assembly.required_functions);

        for (pos, (inst, span)) in assembly.instructions.into_iter().enumerate() {
            let mut comment = None::<Box<str>>;
            let label = assembly.labels_rev.get(&pos).copied();

            match inst {
                AssemblyInst::Jump { label } => {
                    comment = Some(format!("label:{}", label).into());
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::Jump { offset });
                }
                AssemblyInst::JumpIf { label } => {
                    comment = Some(format!("label:{}", label).into());
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::JumpIf { offset });
                }
                AssemblyInst::JumpIfOrPop { label } => {
                    comment = Some(format!("label:{}", label).into());
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::JumpIfOrPop { offset });
                }
                AssemblyInst::JumpIfNotOrPop { label } => {
                    comment = Some(format!("label:{}", label).into());
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::JumpIfNotOrPop { offset });
                }
                AssemblyInst::JumpIfBranch { branch, label } => {
                    comment = Some(format!("label:{}", label).into());
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions
                        .push(Inst::JumpIfBranch { branch, offset });
                }
                AssemblyInst::PopAndJumpIfNot { count, label } => {
                    comment = Some(format!("label:{}", label).into());
                    let offset = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions
                        .push(Inst::PopAndJumpIfNot { count, offset });
                }
                AssemblyInst::IterNext { offset, label } => {
                    comment = Some(format!("label:{}", label).into());
                    let jump = translate_offset(span, pos, label, &assembly.labels)?;
                    self.instructions.push(Inst::IterNext { offset, jump });
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
                comment = Some(actual.into())
            }

            let debug = self.debug.get_or_insert_with(Default::default);

            debug.instructions.push(DebugInst::new(
                location.source_id,
                span,
                comment,
                label.map(Label::into_owned),
            ));
        }

        return Ok(());

        fn translate_offset(
            span: Span,
            base: usize,
            label: Label,
            labels: &HashMap<Label, usize>,
        ) -> Result<isize, CompileError> {
            use std::convert::TryFrom;

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
