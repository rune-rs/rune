//! A single execution unit in the runestick virtual machine.
//!
//! A unit consists of a sequence of instructions, and lookaside tables for
//! metadata like function locations.

use crate::no_std as std;
use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;
use crate::no_std::thiserror;

use thiserror::Error;

use crate::ast::Span;
use crate::collections::HashMap;
use crate::compile::meta;
use crate::compile::{
    self, Assembly, AssemblyInst, CompileErrorKind, Item, Location, Pool, QueryErrorKind, WithSpan,
};
use crate::runtime::debug::{DebugArgs, DebugSignature};
use crate::runtime::unit::UnitEncoder;
use crate::runtime::{
    Call, ConstValue, DebugInfo, DebugInst, Inst, Protocol, Rtti, StaticString, Unit, UnitFn,
    VariantRtti,
};
use crate::{Context, Diagnostics, Hash, SourceId};

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
    /// Convert into a runtime unit, shedding our build metadata in the process.
    ///
    /// Returns `None` if the builder is still in use.
    pub(crate) fn build<S>(mut self, span: Span, storage: S) -> compile::Result<Unit<S>> {
        if let Some(debug) = &mut self.debug {
            debug.functions_rev = self.functions_rev;
        }

        for (from, to) in self.reexports {
            if let Some(info) = self.functions.get(&to) {
                let info = *info;
                if self.functions.insert(from, info).is_some() {
                    return Err(compile::Error::new(
                        span,
                        CompileErrorKind::FunctionConflictHash { hash: from },
                    ));
                }
                continue;
            }

            if let Some(value) = self.constants.get(&to) {
                let const_value = value.clone();

                if self.constants.insert(from, const_value).is_some() {
                    return Err(compile::Error::new(
                        span,
                        CompileErrorKind::ConstantConflict { hash: from },
                    ));
                }

                continue;
            }

            return Err(compile::Error::new(
                span,
                CompileErrorKind::MissingFunctionHash { hash: to },
            ));
        }

        Ok(Unit::new(
            storage,
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
    ) -> compile::Result<usize> {
        let current = StaticString::new(current);
        let hash = current.hash();

        if let Some(existing_slot) = self.static_string_rev.get(&hash).copied() {
            let existing = self.static_strings.get(existing_slot).ok_or_else(|| {
                compile::Error::new(
                    span,
                    CompileErrorKind::StaticStringMissing {
                        hash,
                        slot: existing_slot,
                    },
                )
            })?;

            if ***existing != *current {
                return Err(compile::Error::new(
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
    ) -> compile::Result<usize> {
        let hash = Hash::static_bytes(current);

        if let Some(existing_slot) = self.static_bytes_rev.get(&hash).copied() {
            let existing = self.static_bytes.get(existing_slot).ok_or_else(|| {
                compile::Error::new(
                    span,
                    CompileErrorKind::StaticBytesMissing {
                        hash,
                        slot: existing_slot,
                    },
                )
            })?;

            if &**existing != current {
                return Err(compile::Error::new(
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
    ) -> compile::Result<usize>
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
    ) -> compile::Result<usize> {
        let hash = Hash::object_keys(&current[..]);

        if let Some(existing_slot) = self.static_object_keys_rev.get(&hash).copied() {
            let existing = self.static_object_keys.get(existing_slot).ok_or_else(|| {
                compile::Error::new(
                    span,
                    CompileErrorKind::StaticObjectKeysMissing {
                        hash,
                        slot: existing_slot,
                    },
                )
            })?;

            if *existing != current {
                return Err(compile::Error::new(
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
    pub(crate) fn insert_meta(
        &mut self,
        span: Span,
        meta: &meta::Meta,
        pool: &mut Pool,
    ) -> compile::Result<()> {
        match meta.kind {
            meta::Kind::Type { .. } => {
                let hash = pool.item_type_hash(meta.item_meta.item);

                let rtti = Arc::new(Rtti {
                    hash,
                    item: pool.item(meta.item_meta.item).to_owned(),
                });

                self.constants.insert(
                    Hash::associated_function(hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(rtti.item.to_string()),
                );

                if self.rtti.insert(hash, rtti).is_some() {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::TypeRttiConflict { hash },
                    ));
                }
            }
            meta::Kind::Struct {
                fields: meta::Fields::Empty,
                ..
            } => {
                let info = UnitFn::UnitStruct { hash: meta.hash };

                let signature = DebugSignature::new(
                    pool.item(meta.item_meta.item).to_owned(),
                    DebugArgs::EmptyArgs,
                );

                let rtti = Arc::new(Rtti {
                    hash: meta.hash,
                    item: pool.item(meta.item_meta.item).to_owned(),
                });

                if self.rtti.insert(meta.hash, rtti).is_some() {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::TypeRttiConflict { hash: meta.hash },
                    ));
                }

                if self.functions.insert(meta.hash, info).is_some() {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::FunctionConflict {
                            existing: signature,
                        },
                    ));
                }

                self.constants.insert(
                    Hash::associated_function(meta.hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(signature.path.to_string()),
                );

                self.debug_info_mut().functions.insert(meta.hash, signature);
            }
            meta::Kind::Struct {
                fields: meta::Fields::Unnamed(args),
                ..
            } => {
                let info = UnitFn::TupleStruct {
                    hash: meta.hash,
                    args,
                };

                let signature = DebugSignature::new(
                    pool.item(meta.item_meta.item).to_owned(),
                    DebugArgs::TupleArgs(args),
                );

                let rtti = Arc::new(Rtti {
                    hash: meta.hash,
                    item: pool.item(meta.item_meta.item).to_owned(),
                });

                if self.rtti.insert(meta.hash, rtti).is_some() {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::TypeRttiConflict { hash: meta.hash },
                    ));
                }

                if self.functions.insert(meta.hash, info).is_some() {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::FunctionConflict {
                            existing: signature,
                        },
                    ));
                }

                self.constants.insert(
                    Hash::associated_function(meta.hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(signature.path.to_string()),
                );

                self.debug_info_mut().functions.insert(meta.hash, signature);
            }
            meta::Kind::Struct { .. } => {
                let hash = pool.item_type_hash(meta.item_meta.item);

                let rtti = Arc::new(Rtti {
                    hash,
                    item: pool.item(meta.item_meta.item).to_owned(),
                });

                self.constants.insert(
                    Hash::associated_function(hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(rtti.item.to_string()),
                );

                if self.rtti.insert(hash, rtti).is_some() {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::TypeRttiConflict { hash },
                    ));
                }
            }
            meta::Kind::Variant {
                enum_hash,
                fields: meta::Fields::Empty,
                ..
            } => {
                let rtti = Arc::new(VariantRtti {
                    enum_hash,
                    hash: meta.hash,
                    item: pool.item(meta.item_meta.item).to_owned(),
                });

                if self.variant_rtti.insert(meta.hash, rtti).is_some() {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::VariantRttiConflict { hash: meta.hash },
                    ));
                }

                let info = UnitFn::UnitVariant { hash: meta.hash };

                let signature = DebugSignature::new(
                    pool.item(meta.item_meta.item).to_owned(),
                    DebugArgs::EmptyArgs,
                );

                if self.functions.insert(meta.hash, info).is_some() {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::FunctionConflict {
                            existing: signature,
                        },
                    ));
                }

                self.debug_info_mut().functions.insert(meta.hash, signature);
            }
            meta::Kind::Variant {
                enum_hash,
                fields: meta::Fields::Unnamed(args),
                ..
            } => {
                let rtti = Arc::new(VariantRtti {
                    enum_hash,
                    hash: meta.hash,
                    item: pool.item(meta.item_meta.item).to_owned(),
                });

                if self.variant_rtti.insert(meta.hash, rtti).is_some() {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::VariantRttiConflict { hash: meta.hash },
                    ));
                }

                let info = UnitFn::TupleVariant {
                    hash: meta.hash,
                    args,
                };

                let signature = DebugSignature::new(
                    pool.item(meta.item_meta.item).to_owned(),
                    DebugArgs::TupleArgs(args),
                );

                if self.functions.insert(meta.hash, info).is_some() {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::FunctionConflict {
                            existing: signature,
                        },
                    ));
                }

                self.debug_info_mut().functions.insert(meta.hash, signature);
            }
            meta::Kind::Variant {
                enum_hash,
                fields: meta::Fields::Named(..),
                ..
            } => {
                let hash = pool.item_type_hash(meta.item_meta.item);

                let rtti = Arc::new(VariantRtti {
                    enum_hash,
                    hash,
                    item: pool.item(meta.item_meta.item).to_owned(),
                });

                if self.variant_rtti.insert(hash, rtti).is_some() {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::VariantRttiConflict { hash },
                    ));
                }
            }
            meta::Kind::Enum { .. } => {
                self.constants.insert(
                    Hash::associated_function(meta.hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(pool.item(meta.item_meta.item).to_string()),
                );
            }
            meta::Kind::Macro { .. } => (),
            meta::Kind::Function { .. } => (),
            meta::Kind::AssociatedFunction { .. } => (),
            meta::Kind::Closure { .. } => (),
            meta::Kind::AsyncBlock { .. } => (),
            meta::Kind::Const { ref const_value } => {
                self.constants.insert(
                    pool.item_type_hash(meta.item_meta.item),
                    const_value.clone(),
                );
            }
            meta::Kind::ConstFn { .. } => (),
            meta::Kind::Import { .. } => (),
            meta::Kind::Module { .. } => (),
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
        item: &Item,
        args: usize,
        assembly: Assembly,
        call: Call,
        debug_args: Box<[Box<str>]>,
        unit_encoder: &mut dyn UnitEncoder,
    ) -> compile::Result<()> {
        let offset = unit_encoder.offset();
        let hash = Hash::type_hash(item);

        self.functions_rev.insert(offset, hash);
        let info = UnitFn::Offset { offset, call, args };
        let signature = DebugSignature::new(item.to_owned(), DebugArgs::Named(debug_args));

        if self.functions.insert(hash, info).is_some() {
            return Err(compile::Error::new(
                location.span,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        self.constants.insert(
            Hash::associated_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(signature.path.to_string()),
        );

        self.debug_info_mut().functions.insert(hash, signature);

        self.add_assembly(location, assembly, unit_encoder)?;
        Ok(())
    }

    /// Register a new function re-export.
    pub(crate) fn new_function_reexport(
        &mut self,
        location: Location,
        item: &Item,
        target: &Item,
    ) -> compile::Result<()> {
        let hash = Hash::type_hash(item);
        let target = Hash::type_hash(target);

        if self.reexports.insert(hash, target).is_some() {
            return Err(compile::Error::new(
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
        item: &Item,
        type_hash: Hash,
        name: &str,
        args: usize,
        assembly: Assembly,
        call: Call,
        debug_args: Box<[Box<str>]>,
        unit_storage: &mut dyn UnitEncoder,
    ) -> compile::Result<()> {
        tracing::trace!("instance fn: {}", item);

        let offset = unit_storage.offset();
        let instance_fn = Hash::associated_function(type_hash, name);
        let hash = Hash::type_hash(item);

        let info = UnitFn::Offset { offset, call, args };
        let signature = DebugSignature::new(item.to_owned(), DebugArgs::Named(debug_args));

        if self.functions.insert(instance_fn, info).is_some() {
            return Err(compile::Error::new(
                location.span,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        if self.functions.insert(hash, info).is_some() {
            return Err(compile::Error::new(
                location.span,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        self.constants.insert(
            Hash::associated_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(signature.path.to_string()),
        );

        self.debug_info_mut()
            .functions
            .insert(instance_fn, signature);
        self.functions_rev.insert(offset, hash);
        self.add_assembly(location, assembly, unit_storage)?;
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

    /// Insert and access debug information.
    fn debug_info_mut(&mut self) -> &mut DebugInfo {
        self.debug.get_or_insert_with(Default::default)
    }

    /// Translate the given assembly into instructions.
    fn add_assembly(
        &mut self,
        location: Location,
        assembly: Assembly,
        storage: &mut dyn UnitEncoder,
    ) -> compile::Result<()> {
        self.label_count = assembly.label_count;

        let base = storage.extend_offsets(assembly.labels.len());
        self.required_functions.extend(assembly.required_functions);

        for (offset, (_, labels)) in &assembly.labels {
            for label in labels {
                if let Some(jump) = label.jump() {
                    label.set_jump(storage.label_jump(base, *offset, jump));
                }
            }
        }

        for (pos, (inst, span)) in assembly.instructions.into_iter().enumerate() {
            let mut comment = None::<Box<str>>;

            let mut labels = Vec::new();

            for label in assembly
                .labels
                .get(&pos)
                .map(|e| e.1.as_slice())
                .unwrap_or_default()
            {
                if let Some(index) = label.jump() {
                    storage.mark_offset(index);
                }

                labels.push(label.to_debug_label());
            }

            let at = storage.offset();

            match inst {
                AssemblyInst::Jump { label } => {
                    let jump = label
                        .jump()
                        .ok_or(CompileErrorKind::MissingLabelLocation {
                            name: label.name,
                            index: label.index,
                        })
                        .with_span(location.span)?;
                    comment = Some(format!("label:{}", label).into());
                    storage
                        .encode(Inst::Jump { jump })
                        .with_span(location.span)?;
                }
                AssemblyInst::JumpIf { label } => {
                    let jump = label
                        .jump()
                        .ok_or(CompileErrorKind::MissingLabelLocation {
                            name: label.name,
                            index: label.index,
                        })
                        .with_span(location.span)?;
                    comment = Some(format!("label:{}", label).into());
                    storage
                        .encode(Inst::JumpIf { jump })
                        .with_span(location.span)?;
                }
                AssemblyInst::JumpIfOrPop { label } => {
                    let jump = label
                        .jump()
                        .ok_or(CompileErrorKind::MissingLabelLocation {
                            name: label.name,
                            index: label.index,
                        })
                        .with_span(location.span)?;
                    comment = Some(format!("label:{}", label).into());
                    storage
                        .encode(Inst::JumpIfOrPop { jump })
                        .with_span(location.span)?;
                }
                AssemblyInst::JumpIfNotOrPop { label } => {
                    let jump = label
                        .jump()
                        .ok_or(CompileErrorKind::MissingLabelLocation {
                            name: label.name,
                            index: label.index,
                        })
                        .with_span(location.span)?;
                    comment = Some(format!("label:{}", label).into());
                    storage
                        .encode(Inst::JumpIfNotOrPop { jump })
                        .with_span(location.span)?;
                }
                AssemblyInst::JumpIfBranch { branch, label } => {
                    let jump = label
                        .jump()
                        .ok_or(CompileErrorKind::MissingLabelLocation {
                            name: label.name,
                            index: label.index,
                        })
                        .with_span(location.span)?;
                    comment = Some(format!("label:{}", label).into());
                    storage
                        .encode(Inst::JumpIfBranch { branch, jump })
                        .with_span(location.span)?;
                }
                AssemblyInst::PopAndJumpIfNot { count, label } => {
                    let jump = label
                        .jump()
                        .ok_or(CompileErrorKind::MissingLabelLocation {
                            name: label.name,
                            index: label.index,
                        })
                        .with_span(location.span)?;
                    comment = Some(format!("label:{}", label).into());
                    storage
                        .encode(Inst::PopAndJumpIfNot { count, jump })
                        .with_span(location.span)?;
                }
                AssemblyInst::IterNext { offset, label } => {
                    let jump = label
                        .jump()
                        .ok_or(CompileErrorKind::MissingLabelLocation {
                            name: label.name,
                            index: label.index,
                        })
                        .with_span(location.span)?;
                    comment = Some(format!("label:{}", label).into());
                    storage
                        .encode(Inst::IterNext { offset, jump })
                        .with_span(location.span)?;
                }
                AssemblyInst::Raw { raw } => {
                    storage.encode(raw).with_span(location.span)?;
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

            debug.instructions.insert(
                at,
                DebugInst::new(location.source_id, span, comment, labels),
            );
        }

        Ok(())
    }
}
