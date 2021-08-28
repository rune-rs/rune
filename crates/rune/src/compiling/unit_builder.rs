//! A single execution unit in the runestick virtual machine.
//!
//! A unit consists of a sequence of instructions, and lookaside tables for
//! metadata like function locations.

use crate::collections::HashMap;
use crate::compiling::{Assembly, AssemblyInst};
use crate::{CompileError, CompileErrorKind, Diagnostics};
use runestick::debug::{DebugArgs, DebugSignature};
use runestick::{
    Call, CompileMeta, CompileMetaKind, ConstValue, Context, DebugInfo, DebugInst, Hash, Inst,
    IntoComponent, Item, Label, Location, Protocol, Rtti, Span, StaticString, Unit, UnitFn,
    VariantRtti,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use thiserror::Error;

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("builder not exclusively held")]
    NotExclusivelyHeld,
    #[error("missing function with hash `{hash}`")]
    MissingFunctionHash { hash: Hash },
    #[error("conflicting function already exists `{hash}`")]
    FunctionConflictHash { hash: Hash },
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

        this.prelude("assert_eq", &["test", "assert_eq"]);
        this.prelude("assert", &["test", "assert"]);
        this.prelude("bool", &["bool"]);
        this.prelude("byte", &["byte"]);
        this.prelude("char", &["char"]);
        this.prelude("dbg", &["io", "dbg"]);
        this.prelude("drop", &["mem", "drop"]);
        this.prelude("Err", &["result", "Result", "Err"]);
        this.prelude("file", &["macros", "builtin", "file"]);
        this.prelude("float", &["float"]);
        this.prelude("format", &["fmt", "format"]);
        this.prelude("int", &["int"]);
        this.prelude("is_readable", &["is_readable"]);
        this.prelude("is_writable", &["is_writable"]);
        this.prelude("line", &["macros", "builtin", "line"]);
        this.prelude("None", &["option", "Option", "None"]);
        this.prelude("Object", &["object", "Object"]);
        this.prelude("Ok", &["result", "Result", "Ok"]);
        this.prelude("Option", &["option", "Option"]);
        this.prelude("panic", &["panic"]);
        this.prelude("print", &["io", "print"]);
        this.prelude("println", &["io", "println"]);
        this.prelude("Result", &["result", "Result"]);
        this.prelude("Some", &["option", "Option", "Some"]);
        this.prelude("String", &["string", "String"]);
        this.prelude("stringify", &["stringify"]);
        this.prelude("unit", &["unit"]);
        this.prelude("Vec", &["vec", "Vec"]);

        Self {
            inner: Rc::new(RefCell::new(this)),
        }
    }

    /// Clone the prelude.
    pub(crate) fn prelude(&self) -> HashMap<Box<str>, Item> {
        self.inner.borrow().prelude.clone()
    }

    /// Convert into a runtime unit, shedding our build metadata in the process.
    ///
    /// Returns `None` if the builder is still in use.
    pub fn build(self) -> Result<Unit, BuildError> {
        let inner = match Rc::try_unwrap(self.inner) {
            Ok(inner) => inner,
            Err(..) => return Err(BuildError::NotExclusivelyHeld),
        };

        let mut inner = inner.into_inner();

        if let Some(debug) = &mut inner.debug {
            debug.functions_rev = inner.functions_rev;
        }

        for (from, to) in inner.reexports {
            let info = match inner.functions.get(&to) {
                Some(info) => *info,
                None => return Err(BuildError::MissingFunctionHash { hash: to }),
            };

            if inner.functions.insert(from, info).is_some() {
                return Err(BuildError::FunctionConflictHash { hash: from });
            }
        }

        Ok(Unit::new(
            inner.instructions,
            inner.functions,
            inner.static_strings,
            inner.static_bytes,
            inner.static_object_keys,
            inner.rtti,
            inner.variant_rtti,
            inner.debug,
            inner.constants,
        ))
    }

    /// Insert a static string and return its associated slot that can later be
    /// looked up through [lookup_string][Unit::lookup_string].
    ///
    /// Only uses up space if the static string is unique.
    pub(crate) fn new_static_string(
        &self,
        span: Span,
        current: &str,
    ) -> Result<usize, CompileError> {
        let mut inner = self.inner.borrow_mut();

        let current = StaticString::new(current);
        let hash = current.hash();

        if let Some(existing_slot) = inner.static_string_rev.get(&hash).copied() {
            let existing = inner.static_strings.get(existing_slot).ok_or_else(|| {
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

        let new_slot = inner.static_strings.len();
        inner.static_strings.push(Arc::new(current));
        inner.static_string_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    /// Insert a static byte string and return its associated slot that can
    /// later be looked up through [lookup_bytes][Unit::lookup_bytes].
    ///
    /// Only uses up space if the static byte string is unique.
    pub(crate) fn new_static_bytes(
        &self,
        span: Span,
        current: &[u8],
    ) -> Result<usize, CompileError> {
        let mut inner = self.inner.borrow_mut();

        let hash = Hash::static_bytes(current);

        if let Some(existing_slot) = inner.static_bytes_rev.get(&hash).copied() {
            let existing = inner.static_bytes.get(existing_slot).ok_or_else(|| {
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

        let new_slot = inner.static_bytes.len();
        inner.static_bytes.push(current.to_owned());
        inner.static_bytes_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    /// Insert a new collection of static object keys, or return one already
    /// existing.
    pub(crate) fn new_static_object_keys_iter<I>(
        &self,
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
        &self,
        span: Span,
        current: Box<[String]>,
    ) -> Result<usize, CompileError> {
        let mut inner = self.inner.borrow_mut();

        let hash = Hash::object_keys(&current[..]);

        if let Some(existing_slot) = inner.static_object_keys_rev.get(&hash).copied() {
            let existing = inner.static_object_keys.get(existing_slot).ok_or_else(|| {
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

        let new_slot = inner.static_object_keys.len();
        inner.static_object_keys.push(current);
        inner.static_object_keys_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    /// Declare a new struct.
    pub(crate) fn insert_meta(&self, meta: &CompileMeta) -> Result<(), InsertMetaError> {
        let mut inner = self.inner.borrow_mut();

        match &meta.kind {
            CompileMetaKind::UnitStruct { empty, .. } => {
                let info = UnitFn::UnitStruct { hash: empty.hash };

                let signature = DebugSignature {
                    path: meta.item.item.clone(),
                    args: DebugArgs::EmptyArgs,
                };

                let rtti = Arc::new(Rtti {
                    hash: empty.hash,
                    item: meta.item.item.clone(),
                });

                if inner.rtti.insert(empty.hash, rtti).is_some() {
                    return Err(InsertMetaError::TypeRttiConflict { hash: empty.hash });
                }

                if inner.functions.insert(empty.hash, info).is_some() {
                    return Err(InsertMetaError::FunctionConflict {
                        existing: signature,
                    });
                }

                inner.constants.insert(
                    Hash::instance_function(empty.hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(signature.path.to_string()),
                );

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
                    path: meta.item.item.clone(),
                    args: DebugArgs::TupleArgs(tuple.args),
                };

                let rtti = Arc::new(Rtti {
                    hash: tuple.hash,
                    item: meta.item.item.clone(),
                });

                if inner.rtti.insert(tuple.hash, rtti).is_some() {
                    return Err(InsertMetaError::TypeRttiConflict { hash: tuple.hash });
                }

                if inner.functions.insert(tuple.hash, info).is_some() {
                    return Err(InsertMetaError::FunctionConflict {
                        existing: signature,
                    });
                }

                inner.constants.insert(
                    Hash::instance_function(tuple.hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(signature.path.to_string()),
                );

                inner
                    .debug_info_mut()
                    .functions
                    .insert(tuple.hash, signature);
            }
            CompileMetaKind::Struct { .. } => {
                let hash = Hash::type_hash(&meta.item.item);

                let rtti = Arc::new(Rtti {
                    hash,
                    item: meta.item.item.clone(),
                });

                inner.constants.insert(
                    Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(rtti.item.to_string()),
                );

                if inner.rtti.insert(hash, rtti).is_some() {
                    return Err(InsertMetaError::TypeRttiConflict { hash });
                }
            }
            CompileMetaKind::UnitVariant {
                enum_item, empty, ..
            } => {
                let enum_hash = Hash::type_hash(enum_item);

                let rtti = Arc::new(VariantRtti {
                    enum_hash,
                    hash: empty.hash,
                    item: meta.item.item.clone(),
                });

                if inner.variant_rtti.insert(empty.hash, rtti).is_some() {
                    return Err(InsertMetaError::VariantRttiConflict { hash: empty.hash });
                }

                let info = UnitFn::UnitVariant { hash: empty.hash };

                let signature = DebugSignature {
                    path: meta.item.item.clone(),
                    args: DebugArgs::EmptyArgs,
                };

                if inner.functions.insert(empty.hash, info).is_some() {
                    return Err(InsertMetaError::FunctionConflict {
                        existing: signature,
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
                    item: meta.item.item.clone(),
                });

                if inner.variant_rtti.insert(tuple.hash, rtti).is_some() {
                    return Err(InsertMetaError::VariantRttiConflict { hash: tuple.hash });
                }

                let info = UnitFn::TupleVariant {
                    hash: tuple.hash,
                    args: tuple.args,
                };

                let signature = DebugSignature {
                    path: meta.item.item.clone(),
                    args: DebugArgs::TupleArgs(tuple.args),
                };

                if inner.functions.insert(tuple.hash, info).is_some() {
                    return Err(InsertMetaError::FunctionConflict {
                        existing: signature,
                    });
                }

                inner
                    .debug_info_mut()
                    .functions
                    .insert(tuple.hash, signature);
            }
            CompileMetaKind::StructVariant { enum_item, .. } => {
                let hash = Hash::type_hash(&meta.item.item);
                let enum_hash = Hash::type_hash(enum_item);

                let rtti = Arc::new(VariantRtti {
                    enum_hash,
                    hash,
                    item: meta.item.item.clone(),
                });

                if inner.variant_rtti.insert(hash, rtti).is_some() {
                    return Err(InsertMetaError::VariantRttiConflict { hash });
                }
            }
            CompileMetaKind::Enum { type_hash } => {
                inner.constants.insert(
                    Hash::instance_function(*type_hash, Protocol::INTO_TYPE_NAME),
                    ConstValue::String(meta.item.item.to_string()),
                );
            }
            CompileMetaKind::Function { .. } => (),
            CompileMetaKind::Closure { .. } => (),
            CompileMetaKind::AsyncBlock { .. } => (),
            CompileMetaKind::Const { .. } => (),
            CompileMetaKind::ConstFn { .. } => (),
            CompileMetaKind::Import { .. } => (),
        }

        Ok(())
    }

    /// Construct a new empty assembly associated with the current unit.
    pub(crate) fn new_assembly(&self, location: Location) -> Assembly {
        let label_count = self.inner.borrow().label_count;
        Assembly::new(location, label_count)
    }

    /// Declare a new function at the current instruction pointer.
    pub(crate) fn new_function(
        &self,
        location: Location,
        path: Item,
        args: usize,
        assembly: Assembly,
        call: Call,
        debug_args: Vec<String>,
    ) -> Result<(), CompileError> {
        let mut inner = self.inner.borrow_mut();

        let offset = inner.instructions.len();
        let hash = Hash::type_hash(&path);

        inner.functions_rev.insert(offset, hash);
        let info = UnitFn::Offset { offset, call, args };
        let signature = DebugSignature::new(path, debug_args);

        if inner.functions.insert(hash, info).is_some() {
            return Err(CompileError::new(
                location.span,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        inner.constants.insert(
            Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(signature.path.to_string()),
        );

        inner.debug_info_mut().functions.insert(hash, signature);

        inner.add_assembly(location, assembly)?;
        Ok(())
    }

    /// Register a new function re-export.
    pub(crate) fn new_function_reexport(
        &self,
        location: Location,
        item: &Item,
        target: &Item,
    ) -> Result<(), CompileError> {
        let mut inner = self.inner.borrow_mut();
        let hash = Hash::type_hash(item);
        let target = Hash::type_hash(target);

        if inner.reexports.insert(hash, target).is_some() {
            return Err(CompileError::new(
                location.span,
                CompileErrorKind::FunctionReExportConflict { hash },
            ));
        }

        Ok(())
    }

    /// Declare a new instance function at the current instruction pointer.
    pub(crate) fn new_instance_function(
        &self,
        location: Location,
        path: Item,
        type_hash: Hash,
        name: &str,
        args: usize,
        assembly: Assembly,
        call: Call,
        debug_args: Vec<String>,
    ) -> Result<(), CompileError> {
        log::trace!("instance fn: {}", path);

        let mut inner = self.inner.borrow_mut();

        let offset = inner.instructions.len();
        let instance_fn = Hash::instance_function(type_hash, name);
        let hash = Hash::type_hash(&path);

        let info = UnitFn::Offset { offset, call, args };
        let signature = DebugSignature::new(path, debug_args);

        if inner.functions.insert(instance_fn, info).is_some() {
            return Err(CompileError::new(
                location.span,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        if inner.functions.insert(hash, info).is_some() {
            return Err(CompileError::new(
                location.span,
                CompileErrorKind::FunctionConflict {
                    existing: signature,
                },
            ));
        }

        inner.constants.insert(
            Hash::instance_function(hash, Protocol::INTO_TYPE_NAME),
            ConstValue::String(signature.path.to_string()),
        );

        inner
            .debug_info_mut()
            .functions
            .insert(instance_fn, signature);
        inner.functions_rev.insert(offset, hash);
        inner.add_assembly(location, assembly)?;
        Ok(())
    }

    /// Try to link the unit with the context, checking that all necessary
    /// functions are provided.
    ///
    /// This can prevent a number of runtime errors, like missing functions.
    pub(crate) fn link(&self, context: &Context, diagnostics: &mut Diagnostics) {
        let inner = self.inner.borrow();

        for (hash, spans) in &inner.required_functions {
            if inner.functions.get(hash).is_none() && context.lookup(*hash).is_none() {
                diagnostics.error(
                    0,
                    LinkerError::MissingFunction {
                        hash: *hash,
                        spans: spans.clone(),
                    },
                );
            }
        }
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
    required_functions: HashMap<Hash, Vec<(Span, usize)>>,
    /// Debug info if available for unit.
    debug: Option<Box<DebugInfo>>,

    /// Constant values
    constants: HashMap<Hash, ConstValue>,
}

impl Inner {
    /// Define a prelude item.
    fn prelude<I>(&mut self, local: &str, path: I)
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
                AssemblyInst::IterNext { offset, label } => {
                    comment = Some(format!("label:{}", label));
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
                comment = Some(actual)
            }

            let debug = self.debug.get_or_insert_with(Default::default);

            debug.instructions.push(DebugInst {
                source_id: location.source_id,
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
}
