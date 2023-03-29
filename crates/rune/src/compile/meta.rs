//! Compiler metadata for Rune.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use crate::ast::{LitStr, Span};
use crate::collections::HashSet;
use crate::compile::attrs::Attributes;
use crate::compile::{Docs, Item, ItemBuf, ItemId, Location, ModId, Pool, Visibility};
use crate::parse::{Id, ParseError, ResolveContext};
use crate::query::ImportEntry;
use crate::runtime::{ConstValue, TypeInfo};
use crate::{Hash, InstFnKind, Module};

/// Provides an owned human-readable description of a meta item.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Meta {
    /// The hash of the item.
    pub hash: Hash,
    /// The item being described.
    pub item: ItemBuf,
    /// The kind of the item.
    pub kind: MetaKind,
}

/// Provides a human-readable description of a meta item. This is cheaper to use
/// than [Meta] because it avoids having to clone some data.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct MetaRef<'a> {
    /// The hash of a meta item.
    pub hash: Hash,
    /// The item being described.
    pub item: &'a Item,
    /// The kind of the item.
    pub kind: MetaKind,
    /// The source of the meta.
    pub source: Option<&'a SourceMeta>,
}

/// Describes the kind of a [Meta] or [MetaRef].
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum MetaKind {
    /// An unknown type.
    Unknown,
    /// Item describes a unit structure.
    UnitStruct,
    /// Item describes a tuple structure.
    TupleStruct,
    /// Item describes a regular structure.
    Struct,
    /// Item describes a unit variant.
    UnitVariant,
    /// Item describes a tuple variant.
    TupleVariant,
    /// Item describes a struct variant.
    StructVariant,
    /// Item describes an enum.
    Enum,
    /// Item describes a function.
    Function {
        /// The number of arguments the function takes.
        args: Option<usize>,
        /// If the function is a test.
        is_test: bool,
        /// If the function is a benchmark.
        is_bench: bool,
    },
    /// Item describes a closure.
    Closure,
    /// Item describes an async block.
    AsyncBlock,
    /// Item describes a constant.
    Const,
    /// Item describes a constant function.
    ConstFn,
    /// Item describes an import.
    Import,
    /// Item describes a module.
    Module,
}

impl fmt::Display for Meta {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            MetaKind::Unknown => {
                write!(fmt, "unknown {}", self.item)?;
            }
            MetaKind::UnitStruct => {
                write!(fmt, "struct {}", self.item)?;
            }
            MetaKind::TupleStruct => {
                write!(fmt, "struct {}", self.item)?;
            }
            MetaKind::Struct => {
                write!(fmt, "struct {}", self.item)?;
            }
            MetaKind::UnitVariant => {
                write!(fmt, "unit variant {}", self.item)?;
            }
            MetaKind::TupleVariant => {
                write!(fmt, "variant {}", self.item)?;
            }
            MetaKind::StructVariant => {
                write!(fmt, "variant {}", self.item)?;
            }
            MetaKind::Enum => {
                write!(fmt, "enum {}", self.item)?;
            }
            MetaKind::Function { .. } => {
                write!(fmt, "fn {}", self.item)?;
            }
            MetaKind::Closure => {
                write!(fmt, "closure {}", self.item)?;
            }
            MetaKind::AsyncBlock => {
                write!(fmt, "async block {}", self.item)?;
            }
            MetaKind::Const => {
                write!(fmt, "const {}", self.item)?;
            }
            MetaKind::ConstFn => {
                write!(fmt, "const fn {}", self.item)?;
            }
            MetaKind::Import => {
                write!(fmt, "import {}", self.item)?;
            }
            MetaKind::Module => {
                write!(fmt, "module {}", self.item)?;
            }
        }

        Ok(())
    }
}

/// Information on a compile sourc.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SourceMeta {
    /// The location of the compile source.
    pub location: Location,
    /// The optional path where the meta is declared.
    pub path: Option<Box<Path>>,
}

/// Metadata about a variable captured by a clsoreu.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) struct CaptureMeta {
    /// Identity of the captured variable.
    pub(crate) ident: Box<str>,
}

/// Doc content for a compiled item.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Doc {
    /// The span of the whole doc comment.
    pub(crate) span: Span,
    /// The string content of the doc comment.
    pub(crate) doc_string: LitStr,
}

impl Doc {
    pub(crate) fn collect_from(
        ctx: ResolveContext<'_>,
        attrs: &mut Attributes,
    ) -> Result<Vec<Doc>, ParseError> {
        Ok(attrs
            .try_parse_collect::<crate::compile::attrs::Doc>(ctx)?
            .into_iter()
            .map(|(span, doc)| Doc {
                span,
                doc_string: doc.doc_string,
            })
            .collect())
    }
}

/// Context metadata.
#[non_exhaustive]
pub struct ContextMeta {
    /// The module that the declared item belongs to.
    #[cfg(feature = "doc")]
    pub module: ItemBuf,
    /// Type hash for the given meta item.
    pub hash: Hash,
    /// The item of the returned compile meta.
    pub item: ItemBuf,
    /// The kind of the compile meta.
    pub kind: ContextMetaKind,
    /// Documentation associated with a context meta.
    pub docs: Docs,
}

impl ContextMeta {
    pub(crate) fn new(
        module: &Module,
        hash: Hash,
        item: ItemBuf,
        kind: ContextMetaKind,
        docs: Docs,
    ) -> Self {
        Self {
            #[cfg(feature = "doc")]
            module: module.item.clone(),
            hash,
            item,
            kind,
            docs,
        }
    }

    /// Get the [Meta] which describes this [ContextMeta] object.
    pub(crate) fn info(&self) -> Meta {
        Meta {
            hash: self.hash,
            item: self.item.clone(),
            kind: self.kind.as_meta_info_kind(),
        }
    }
}

/// Compile-time metadata kind about an item in a context.
#[derive(Debug, Clone)]
pub enum ContextMetaKind {
    /// The type is completely opaque. We have no idea about what it is with the
    /// exception of it having a type hash.
    Unknown,
    /// Metadata about a struct.
    Struct {
        /// Variant metadata.
        variant: PrivVariantMeta,
    },
    /// Metadata about an empty variant.
    Variant {
        /// The item of the enum.
        enum_item: ItemBuf,
        /// Type hash of the enum this unit variant belongs to.
        enum_hash: Hash,
        /// The index of the variant.
        index: usize,
        /// Variant metadata.
        variant: PrivVariantMeta,
    },
    /// An enum item.
    Enum,
    /// A function declaration.
    Function {
        /// Number of arguments this function takes.
        args: Option<usize>,
        /// Indicates if the function is an instance function or not.
        instance_function: bool,
    },
    /// The constant expression.
    Const {
        /// The evaluated constant value.
        const_value: ConstValue,
    },
}

impl ContextMetaKind {
    /// Coerce into a [MetaKind].
    pub(crate) fn as_meta_info_kind(&self) -> MetaKind {
        match self {
            ContextMetaKind::Unknown { .. } => MetaKind::Unknown,
            ContextMetaKind::Struct {
                variant: PrivVariantMeta::Unit,
                ..
            } => MetaKind::UnitStruct,
            ContextMetaKind::Struct {
                variant: PrivVariantMeta::Tuple(..),
                ..
            } => MetaKind::TupleStruct,
            ContextMetaKind::Struct {
                variant: PrivVariantMeta::Struct(..),
                ..
            } => MetaKind::Struct,
            ContextMetaKind::Variant {
                variant: PrivVariantMeta::Unit,
                ..
            } => MetaKind::UnitVariant,
            ContextMetaKind::Variant {
                variant: PrivVariantMeta::Tuple(..),
                ..
            } => MetaKind::TupleVariant,
            ContextMetaKind::Variant {
                variant: PrivVariantMeta::Struct(..),
                ..
            } => MetaKind::StructVariant,
            ContextMetaKind::Enum { .. } => MetaKind::Enum,
            ContextMetaKind::Function { args, .. } => MetaKind::Function {
                args: *args,
                is_bench: false,
                is_test: false,
            },
            ContextMetaKind::Const { .. } => MetaKind::Const,
        }
    }
}

/// Metadata about a compiled unit.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) struct PrivMeta {
    /// Hash of the private metadata.
    pub(crate) hash: Hash,
    /// The item of the returned compile meta.
    pub(crate) item_meta: ItemMeta,
    /// The kind of the compile meta.
    pub(crate) kind: PrivMetaKind,
    /// The source of the meta.
    pub(crate) source: Option<SourceMeta>,
}

impl PrivMeta {
    /// Get the [Meta] which describes this [ContextMeta] object.
    pub(crate) fn info(&self, pool: &Pool) -> Meta {
        Meta {
            hash: self.hash,
            item: pool.item(self.item_meta.item).to_owned(),
            kind: self.kind.as_meta_info_kind(),
        }
    }

    /// Get the [MetaRef] which describes this [PrivMeta] object.
    pub(crate) fn as_meta_ref<'a>(&'a self, pool: &'a Pool) -> MetaRef<'a> {
        MetaRef {
            hash: self.hash,
            item: pool.item(self.item_meta.item),
            kind: self.kind.as_meta_info_kind(),
            source: self.source.as_ref(),
        }
    }

    /// Get the type hash of the base type (the one to type check for) for the
    /// given compile meta.
    ///
    /// Note: Variants cannot be used for type checking, you should instead
    /// compare them against the enum type.
    pub(crate) fn type_hash_of(&self) -> Option<Hash> {
        match &self.kind {
            PrivMetaKind::Unknown { .. } => Some(self.hash),
            PrivMetaKind::Struct { .. } => Some(self.hash),
            PrivMetaKind::Enum { .. } => Some(self.hash),
            PrivMetaKind::Function { .. } => Some(self.hash),
            PrivMetaKind::Closure { .. } => Some(self.hash),
            PrivMetaKind::AsyncBlock { .. } => Some(self.hash),
            PrivMetaKind::Variant { .. } => None,
            PrivMetaKind::Const { .. } => None,
            PrivMetaKind::ConstFn { .. } => None,
            PrivMetaKind::Import { .. } => None,
            PrivMetaKind::Module => None,
        }
    }
}

/// Private variant metadata.
#[derive(Debug, Clone)]
pub enum PrivVariantMeta {
    Tuple(PrivTupleMeta),
    Struct(PrivStructMeta),
    Unit,
}

/// Compile-time metadata kind about a unit.
#[derive(Debug, Clone)]
pub(crate) enum PrivMetaKind {
    /// The type is completely opaque. We have no idea about what it is with the
    /// exception of it having a type hash.
    Unknown,
    /// Metadata about a struct.
    Struct {
        /// Variant metadata.
        variant: PrivVariantMeta,
    },
    /// Metadata about an empty variant.
    Variant {
        /// The item of the enum.
        enum_item: ItemId,
        /// Type hash of the enum this unit variant belongs to.
        enum_hash: Hash,
        /// The index of the variant.
        index: usize,
        /// Variant metadata.
        variant: PrivVariantMeta,
    },
    /// An enum item.
    Enum,
    /// A function declaration.
    Function {
        /// The number of arguments the function takes.
        args: Option<usize>,
        /// Whether this function has a `#[test]` annotation
        is_test: bool,
        /// Whether this function has a `#[bench]` annotation.
        is_bench: bool,
        /// Indicates that the function is an instance function.
        instance_function: bool,
    },
    /// A closure.
    Closure {
        /// Sequence of captured variables.
        captures: Arc<[CaptureMeta]>,
        /// If the closure moves its environment.
        do_move: bool,
    },
    /// An async block.
    AsyncBlock {
        /// Sequence of captured variables.
        captures: Arc<[CaptureMeta]>,
        /// If the async block moves its environment.
        do_move: bool,
    },
    /// The constant expression.
    Const {
        /// The evaluated constant value.
        const_value: ConstValue,
    },
    /// A constant function.
    ConstFn {
        /// Opaque identifier for the constant function.
        id: Id,
    },
    /// Purely an import.
    Import {
        /// The entry being imported.
        import: ImportEntry,
    },
    /// A module.
    Module,
}

impl PrivMetaKind {
    /// Coerce into a [MetaKind].
    pub(crate) fn as_meta_info_kind(&self) -> MetaKind {
        match self {
            PrivMetaKind::Unknown { .. } => MetaKind::Unknown,
            PrivMetaKind::Struct {
                variant: PrivVariantMeta::Unit,
                ..
            } => MetaKind::UnitStruct,
            PrivMetaKind::Struct {
                variant: PrivVariantMeta::Tuple(..),
                ..
            } => MetaKind::TupleStruct,
            PrivMetaKind::Struct {
                variant: PrivVariantMeta::Struct(..),
                ..
            } => MetaKind::Struct,
            PrivMetaKind::Variant {
                variant: PrivVariantMeta::Unit,
                ..
            } => MetaKind::UnitVariant,
            PrivMetaKind::Variant {
                variant: PrivVariantMeta::Tuple(..),
                ..
            } => MetaKind::TupleVariant,
            PrivMetaKind::Variant {
                variant: PrivVariantMeta::Struct(..),
                ..
            } => MetaKind::StructVariant,
            PrivMetaKind::Enum { .. } => MetaKind::Enum,
            PrivMetaKind::Function {
                args,
                is_bench,
                is_test,
                ..
            } => MetaKind::Function {
                args: *args,
                is_bench: *is_bench,
                is_test: *is_test,
            },
            PrivMetaKind::Closure { .. } => MetaKind::Closure,
            PrivMetaKind::AsyncBlock { .. } => MetaKind::AsyncBlock,
            PrivMetaKind::Const { .. } => MetaKind::Const,
            PrivMetaKind::ConstFn { .. } => MetaKind::ConstFn,
            PrivMetaKind::Import { .. } => MetaKind::Import,
            PrivMetaKind::Module => MetaKind::Module,
        }
    }
}

/// The metadata about a struct.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PrivStructMeta {
    /// Fields associated with the type.
    pub(crate) fields: HashSet<Box<str>>,
}

/// The metadata about a tuple.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PrivTupleMeta {
    /// The number of arguments the variant takes.
    pub(crate) args: usize,
    /// Hash of the constructor function.
    pub(crate) hash: Hash,
}

/// Item and the module that the item belongs to.
#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ItemMeta {
    /// The id of the item.
    pub(crate) id: Id,
    /// The location of the item.
    pub(crate) location: Location,
    /// The name of the item.
    pub(crate) item: ItemId,
    /// The visibility of the item.
    pub(crate) visibility: Visibility,
    /// The module associated with the item.
    pub(crate) module: ModId,
}

impl ItemMeta {
    /// Test if the item is public (and should be exported).
    pub(crate) fn is_public(&self, pool: &Pool) -> bool {
        self.visibility.is_public() && pool.module(self.module).is_public(pool)
    }
}

/// Public type information.
#[non_exhaustive]
pub struct ContextTypeInfo<'a> {
    /// The item of the type.
    pub item: &'a Item,
}

impl fmt::Display for ContextTypeInfo<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.item.fmt(f)
    }
}

/// A description of a function signature.
#[derive(Debug, Clone)]
pub enum ContextSignature {
    /// An unbound or static function
    Function {
        /// The type hash of the function
        type_hash: Hash,
        /// Path to the function.
        item: ItemBuf,
        /// Arguments.
        args: Option<usize>,
    },
    /// An instance function or method
    Instance {
        /// The type hash of the function
        type_hash: Hash,
        /// Path to the instance function.
        item: ItemBuf,
        /// Name of the instance function.
        name: InstFnKind,
        /// Arguments.
        args: Option<usize>,
        /// Information on the self type.
        self_type_info: TypeInfo,
    },
}

impl fmt::Display for ContextSignature {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Function { item, args, .. } => {
                write!(fmt, "{}(", item)?;

                if let Some(args) = args {
                    let mut it = 0..*args;
                    let last = it.next_back();

                    for n in it {
                        write!(fmt, "#{}, ", n)?;
                    }

                    if let Some(n) = last {
                        write!(fmt, "#{}", n)?;
                    }
                } else {
                    write!(fmt, "...")?;
                }

                write!(fmt, ")")?;
            }
            Self::Instance {
                item,
                name,
                self_type_info,
                args,
                ..
            } => {
                write!(fmt, "{}::{}(self: {}", item, name, self_type_info)?;

                if let Some(args) = args {
                    for n in 0..*args {
                        write!(fmt, ", #{}", n)?;
                    }
                } else {
                    write!(fmt, ", ...")?;
                }

                write!(fmt, ")")?;
            }
        }

        Ok(())
    }
}
