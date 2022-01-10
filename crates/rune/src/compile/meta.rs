//! Compiler metadata for Rune.

use crate::collections::HashSet;
use crate::compile::{Item, Location, Visibility};
use crate::parse::Id;
use crate::runtime::ConstValue;
use crate::Hash;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

/// Provides an owned human-readable description of a meta item.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Meta {
    /// The item being described.
    pub item: Item,
    /// The kind of the item.
    pub kind: MetaKind,
}

/// Provides a human-readable description of a meta item. This is cheaper to use
/// than [Meta] because it avoids having to clone some data.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct MetaRef<'a> {
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
        /// The type hash of the function.
        type_hash: Hash,
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

/// Metadata about a compiled unit.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) struct PrivMeta {
    /// The item of the returned compile meta.
    pub(crate) item: Arc<ItemMeta>,
    /// The kind of the compile meta.
    pub(crate) kind: PrivMetaKind,
    /// The source of the meta.
    pub(crate) source: Option<SourceMeta>,
}

impl PrivMeta {
    /// Get the [Meta] which describes this [PrivMeta] object.
    pub(crate) fn info(&self) -> Meta {
        Meta {
            item: self.item.item.clone(),
            kind: self.kind.as_meta_info_kind(),
        }
    }

    /// Get the [MetaRef] which describes this [PrivMeta] object.
    pub(crate) fn info_ref(&self) -> MetaRef<'_> {
        MetaRef {
            item: &self.item.item,
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
            PrivMetaKind::Unknown { type_hash, .. } => Some(*type_hash),
            PrivMetaKind::Struct { type_hash, .. } => Some(*type_hash),
            PrivMetaKind::Enum { type_hash, .. } => Some(*type_hash),
            PrivMetaKind::Function { type_hash, .. } => Some(*type_hash),
            PrivMetaKind::Closure { type_hash, .. } => Some(*type_hash),
            PrivMetaKind::AsyncBlock { type_hash, .. } => Some(*type_hash),
            PrivMetaKind::Variant { .. } => None,
            PrivMetaKind::Const { .. } => None,
            PrivMetaKind::ConstFn { .. } => None,
            PrivMetaKind::Import { .. } => None,
        }
    }
}

/// Private variant metadata.
#[derive(Debug, Clone)]
pub(crate) enum PrivVariantMeta {
    Tuple(PrivTupleMeta),
    Struct(PrivStructMeta),
    Unit,
}

/// Compile-time metadata kind about a unit.
#[derive(Debug, Clone)]
pub(crate) enum PrivMetaKind {
    /// The type is completely opaque. We have no idea about what it is with the
    /// exception of it having a type hash.
    Unknown { type_hash: Hash },
    /// Metadata about a struct.
    Struct {
        /// The type hash associated with this meta kind.
        type_hash: Hash,
        /// Variant metadata.
        variant: PrivVariantMeta,
    },
    /// Metadata about an empty variant.
    Variant {
        /// The type hash associated with this meta kind.
        type_hash: Hash,
        /// The item of the enum.
        enum_item: Item,
        /// Type hash of the enum this unit variant belongs to.
        enum_hash: Hash,
        /// The index of the variant.
        index: usize,
        /// Variant metadata.
        variant: PrivVariantMeta,
    },
    /// An enum item.
    Enum {
        /// The type hash associated with this meta kind.
        type_hash: Hash,
    },
    /// A function declaration.
    Function {
        /// The type hash associated with this meta kind.
        type_hash: Hash,

        /// Whether this function has a `#[test]` annotation
        is_test: bool,

        /// Whether this function has a `#[bench]` annotation.
        is_bench: bool,
    },
    /// A closure.
    Closure {
        /// The type hash associated with this meta kind.
        type_hash: Hash,
        /// Sequence of captured variables.
        captures: Arc<[CaptureMeta]>,
        /// If the closure moves its environment.
        do_move: bool,
    },
    /// An async block.
    AsyncBlock {
        /// The span where the async block is declared.
        type_hash: Hash,
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
        /// The module of the target.
        module: Arc<ModMeta>,
        /// The location of the import.
        location: Location,
        /// The imported target.
        target: Item,
    },
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
                type_hash,
                is_bench,
                is_test,
                ..
            } => MetaKind::Function {
                type_hash: *type_hash,
                is_bench: *is_bench,
                is_test: *is_test,
            },
            PrivMetaKind::Closure { .. } => MetaKind::Closure,
            PrivMetaKind::AsyncBlock { .. } => MetaKind::AsyncBlock,
            PrivMetaKind::Const { .. } => MetaKind::Const,
            PrivMetaKind::ConstFn { .. } => MetaKind::ConstFn,
            PrivMetaKind::Import { .. } => MetaKind::Import,
        }
    }
}

/// The metadata about a struct.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) struct PrivStructMeta {
    /// Fields associated with the type.
    pub(crate) fields: HashSet<Box<str>>,
}

/// The metadata about a tuple.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) struct PrivTupleMeta {
    /// The number of arguments the variant takes.
    pub(crate) args: usize,
    /// Hash of the constructor function.
    pub(crate) hash: Hash,
}

/// Item and the module that the item belongs to.
#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub(crate) struct ItemMeta {
    /// The id of the item.
    pub(crate) id: Id,
    /// The location of the item.
    pub(crate) location: Location,
    /// The name of the item.
    pub(crate) item: Item,
    /// The visibility of the item.
    pub(crate) visibility: Visibility,
    /// The module associated with the item.
    pub(crate) module: Arc<ModMeta>,
}

impl ItemMeta {
    /// Test if the item is public (and should be exported).
    pub(crate) fn is_public(&self) -> bool {
        self.visibility.is_public() && self.module.is_public()
    }
}

impl From<Item> for ItemMeta {
    fn from(item: Item) -> Self {
        Self {
            id: Default::default(),
            location: Default::default(),
            item,
            visibility: Default::default(),
            module: Default::default(),
        }
    }
}

/// Module, its item and its visibility.
#[derive(Default, Debug)]
#[non_exhaustive]
pub(crate) struct ModMeta {
    /// The location of the module.
    pub(crate) location: Location,
    /// The item of the module.
    pub(crate) item: Item,
    /// The visibility of the module.
    pub(crate) visibility: Visibility,
    /// The kind of the module.
    pub(crate) parent: Option<Arc<ModMeta>>,
}

impl ModMeta {
    /// Test if the module recursively is public.
    pub(crate) fn is_public(&self) -> bool {
        let mut current = Some(self);

        while let Some(m) = current.take() {
            if !m.visibility.is_public() {
                return false;
            }

            current = m.parent.as_deref();
        }

        true
    }
}
