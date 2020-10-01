use crate::collections::HashSet;
use crate::{ConstValue, Hash, Id, Item, SourceId, Span, Type};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

/// Metadata about a closure.
#[derive(Debug, Clone)]
pub struct CompileMetaCapture {
    /// Identity of the captured variable.
    pub ident: String,
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
    /// The span where the meta is declared.
    pub span: Span,
    /// The optional source id where the meta is declared.
    pub path: Option<PathBuf>,
    /// The source id where the compile meta is defined.
    pub source_id: SourceId,
}

impl CompileMeta {
    /// Get the value type of the meta item.
    pub fn type_of(&self) -> Option<Type> {
        match &self.kind {
            CompileMetaKind::UnitStruct { type_of, .. } => Some(*type_of),
            CompileMetaKind::TupleStruct { type_of, .. } => Some(*type_of),
            CompileMetaKind::Struct { type_of, .. } => Some(*type_of),
            CompileMetaKind::Enum { type_of, .. } => Some(*type_of),
            CompileMetaKind::Function { type_of, .. } => Some(*type_of),
            CompileMetaKind::Closure { type_of, .. } => Some(*type_of),
            CompileMetaKind::AsyncBlock { type_of, .. } => Some(*type_of),
            CompileMetaKind::UnitVariant { .. } => None,
            CompileMetaKind::TupleVariant { .. } => None,
            CompileMetaKind::StructVariant { .. } => None,
            CompileMetaKind::Macro { .. } => None,
            CompileMetaKind::Const { .. } => None,
            CompileMetaKind::ConstFn { .. } => None,
            CompileMetaKind::Import { .. } => None,
        }
    }
}

impl fmt::Display for CompileMeta {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            CompileMetaKind::Import { .. } => {
                write!(fmt, "import {}", self.item)?;
            }
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
            CompileMetaKind::Macro => {
                write!(fmt, "macro {}", self.item)?;
            }
            CompileMetaKind::Const { .. } => {
                write!(fmt, "const {}", self.item)?;
            }
            CompileMetaKind::ConstFn { .. } => {
                write!(fmt, "const fn {}", self.item)?;
            }
        }

        Ok(())
    }
}

/// Compile-time metadata kind about a unit.
#[derive(Debug, Clone)]
pub enum CompileMetaKind {
    /// An imported item.
    ///
    /// This is the result of a use statement.
    Import {
        /// The imported item.
        import: Item,
    },
    /// Metadata about an object.
    UnitStruct {
        /// The value type associated with this meta item.
        type_of: Type,
        /// The underlying object.
        empty: CompileMetaEmpty,
    },
    /// Metadata about a tuple.
    TupleStruct {
        /// The value type associated with this meta item.
        type_of: Type,
        /// The underlying tuple.
        tuple: CompileMetaTuple,
    },
    /// Metadata about an object.
    Struct {
        /// The value type associated with this meta item.
        type_of: Type,
        /// The underlying object.
        object: CompileMetaStruct,
    },
    /// Metadata about an empty variant.
    UnitVariant {
        /// The value type associated with this meta item.
        type_of: Type,
        /// The item of the enum.
        enum_item: Item,
        /// The underlying empty.
        empty: CompileMetaEmpty,
    },
    /// Metadata about a tuple variant.
    TupleVariant {
        /// The value type associated with this meta item.
        type_of: Type,
        /// The item of the enum.
        enum_item: Item,
        /// The underlying tuple.
        tuple: CompileMetaTuple,
    },
    /// Metadata about a variant object.
    StructVariant {
        /// The value type associated with this meta item.
        type_of: Type,
        /// The item of the enum.
        enum_item: Item,
        /// The underlying object.
        object: CompileMetaStruct,
    },
    /// An enum item.
    Enum {
        /// The value type associated with this meta item.
        type_of: Type,
    },
    /// A function declaration.
    Function {
        /// The value type associated with this meta item.
        type_of: Type,
    },
    /// A closure.
    Closure {
        /// The value type associated with this meta item.
        type_of: Type,
        /// Sequence of captured variables.
        captures: Arc<Vec<CompileMetaCapture>>,
    },
    /// An async block.
    AsyncBlock {
        /// The span where the async block is declared.
        type_of: Type,
        /// Sequence of captured variables.
        captures: Arc<Vec<CompileMetaCapture>>,
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
    /// A macro.
    Macro,
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
    pub fields: Option<HashSet<String>>,
}

/// The metadata about a variant.
#[derive(Debug, Clone)]
pub struct CompileMetaTuple {
    /// The number of arguments the variant takes.
    pub args: usize,
    /// Hash of the constructor function.
    pub hash: Hash,
}
