use crate::collections::HashSet;
use crate::{ConstValue, Hash, Id, Item, SourceId, Span};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

/// Metadata about a closure.
#[derive(Debug, Clone)]
pub struct CompileMetaCapture {
    /// Identity of the captured variable.
    pub ident: Box<str>,
}

/// Compile-time metadata about a unit.
#[derive(Debug, Clone)]
pub struct CompileMeta {
    /// The item of the returned compile meta.
    pub item: Item,
    /// The kind of the compile meta.
    pub kind: CompileMetaKind,
    /// The source of the meta.
    pub source: Option<CompileSource>,
}

/// Information on a compile sourc.
#[derive(Debug, Clone)]
pub struct CompileSource {
    /// The source id where the compile meta is defined.
    pub source_id: SourceId,
    /// The span where the meta is declared.
    pub span: Span,
    /// The optional source id where the meta is declared.
    pub path: Option<PathBuf>,
}

impl CompileMeta {
    /// Get the type hash of the base type (the one to type check for) for the
    /// given compile meta.
    ///
    /// Note: Variants cannot be used for type checking, you should instead
    /// compare them against the enum type.
    pub fn type_hash_of(&self) -> Option<Hash> {
        match &self.kind {
            CompileMetaKind::UnitStruct { type_hash, .. } => Some(*type_hash),
            CompileMetaKind::TupleStruct { type_hash, .. } => Some(*type_hash),
            CompileMetaKind::Struct { type_hash, .. } => Some(*type_hash),
            CompileMetaKind::Enum { type_hash, .. } => Some(*type_hash),
            CompileMetaKind::Function { type_hash, .. } => Some(*type_hash),
            CompileMetaKind::Closure { type_hash, .. } => Some(*type_hash),
            CompileMetaKind::AsyncBlock { type_hash, .. } => Some(*type_hash),
            CompileMetaKind::UnitVariant { .. } => None,
            CompileMetaKind::TupleVariant { .. } => None,
            CompileMetaKind::StructVariant { .. } => None,
            CompileMetaKind::Const { .. } => None,
            CompileMetaKind::ConstFn { .. } => None,
            CompileMetaKind::Import { .. } => None,
        }
    }
}

impl fmt::Display for CompileMeta {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            CompileMetaKind::UnitStruct { .. } => {
                write!(fmt, "struct {}", self.item)?;
            }
            CompileMetaKind::TupleStruct { .. } => {
                write!(fmt, "struct {}", self.item)?;
            }
            CompileMetaKind::Struct { .. } => {
                write!(fmt, "struct {}", self.item)?;
            }
            CompileMetaKind::UnitVariant { .. } => {
                write!(fmt, "unit variant {}", self.item)?;
            }
            CompileMetaKind::TupleVariant { .. } => {
                write!(fmt, "variant {}", self.item)?;
            }
            CompileMetaKind::StructVariant { .. } => {
                write!(fmt, "variant {}", self.item)?;
            }
            CompileMetaKind::Enum { .. } => {
                write!(fmt, "enum {}", self.item)?;
            }
            CompileMetaKind::Function { .. } => {
                write!(fmt, "fn {}", self.item)?;
            }
            CompileMetaKind::Closure { .. } => {
                write!(fmt, "closure {}", self.item)?;
            }
            CompileMetaKind::AsyncBlock { .. } => {
                write!(fmt, "async block {}", self.item)?;
            }
            CompileMetaKind::Const { .. } => {
                write!(fmt, "const {}", self.item)?;
            }
            CompileMetaKind::ConstFn { .. } => {
                write!(fmt, "const fn {}", self.item)?;
            }
            CompileMetaKind::Import { .. } => {
                write!(fmt, "import {}", self.item)?;
            }
        }

        Ok(())
    }
}

/// Compile-time metadata kind about a unit.
#[derive(Debug, Clone)]
pub enum CompileMetaKind {
    /// Metadata about an object.
    UnitStruct {
        /// The type hash associated with this meta kind.
        type_hash: Hash,
        /// The underlying object.
        empty: CompileMetaEmpty,
    },
    /// Metadata about a tuple.
    TupleStruct {
        /// The type hash associated with this meta kind.
        type_hash: Hash,
        /// The underlying tuple.
        tuple: CompileMetaTuple,
    },
    /// Metadata about an object.
    Struct {
        /// The type hash associated with this meta kind.
        type_hash: Hash,
        /// The underlying object.
        object: CompileMetaStruct,
    },
    /// Metadata about an empty variant.
    UnitVariant {
        /// The type hash associated with this meta kind.
        type_hash: Hash,
        /// The item of the enum.
        enum_item: Item,
        /// The underlying empty.
        empty: CompileMetaEmpty,
    },
    /// Metadata about a tuple variant.
    TupleVariant {
        /// The type hash associated with this meta item.
        type_hash: Hash,
        /// The item of the enum.
        enum_item: Item,
        /// The underlying tuple.
        tuple: CompileMetaTuple,
    },
    /// Metadata about a variant object.
    StructVariant {
        /// The type hash associated with this meta kind.
        type_hash: Hash,
        /// The item of the enum.
        enum_item: Item,
        /// The underlying object.
        object: CompileMetaStruct,
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
    },
    /// A closure.
    Closure {
        /// The type hash associated with this meta kind.
        type_hash: Hash,
        /// Sequence of captured variables.
        captures: Arc<[CompileMetaCapture]>,
        /// If the closure moves its environment.
        do_move: bool,
    },
    /// An async block.
    AsyncBlock {
        /// The span where the async block is declared.
        type_hash: Hash,
        /// Sequence of captured variables.
        captures: Arc<[CompileMetaCapture]>,
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
        /// The imported target.
        target: Item,
    },
}

/// The metadata about a type.
#[derive(Debug, Clone)]
pub struct CompileMetaEmpty {
    /// Hash of the constructor function.
    pub hash: Hash,
}

/// The metadata about a type.
#[derive(Debug, Clone)]
pub struct CompileMetaStruct {
    /// Fields associated with the type.
    pub fields: HashSet<Box<str>>,
}

/// The metadata about a variant.
#[derive(Debug, Clone)]
pub struct CompileMetaTuple {
    /// The number of arguments the variant takes.
    pub args: usize,
    /// Hash of the constructor function.
    pub hash: Hash,
}
