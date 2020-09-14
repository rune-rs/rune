use crate::collections::HashSet;
use crate::{Hash, Item, Span, Type, Url};
use std::fmt;
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
    /// The span where the meta is declared.
    pub span: Option<Span>,
    /// The optional source id where the meta is declared.
    pub url: Option<Url>,
    /// The kind of the compile meta.
    pub kind: CompileMetaKind,
}

impl CompileMeta {
    /// Get the item of the meta.
    pub fn item(&self) -> &Item {
        match &self.kind {
            CompileMetaKind::Tuple { tuple, .. } => &tuple.item,
            CompileMetaKind::TupleVariant { tuple, .. } => &tuple.item,
            CompileMetaKind::Struct { object, .. } => &object.item,
            CompileMetaKind::StructVariant { object, .. } => &object.item,
            CompileMetaKind::Enum { item, .. } => item,
            CompileMetaKind::Function { item, .. } => item,
            CompileMetaKind::Closure { item, .. } => item,
            CompileMetaKind::AsyncBlock { item, .. } => item,
            CompileMetaKind::Macro { item, .. } => item,
        }
    }

    /// Get the value type of the meta item.
    pub fn type_of(&self) -> Option<Type> {
        match &self.kind {
            CompileMetaKind::Tuple { type_of, .. } => Some(*type_of),
            CompileMetaKind::TupleVariant { .. } => None,
            CompileMetaKind::Struct { type_of, .. } => Some(*type_of),
            CompileMetaKind::StructVariant { .. } => None,
            CompileMetaKind::Enum { type_of, .. } => Some(*type_of),
            CompileMetaKind::Function { type_of, .. } => Some(*type_of),
            CompileMetaKind::Closure { type_of, .. } => Some(*type_of),
            CompileMetaKind::AsyncBlock { type_of, .. } => Some(*type_of),
            CompileMetaKind::Macro { .. } => None,
        }
    }
}

impl fmt::Display for CompileMeta {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            CompileMetaKind::Tuple { tuple, .. } => {
                write!(fmt, "struct {}", tuple.item)?;
            }
            CompileMetaKind::TupleVariant { tuple, .. } => {
                write!(fmt, "variant {}", tuple.item)?;
            }
            CompileMetaKind::Struct { object, .. } => {
                write!(fmt, "struct {}", object.item)?;
            }
            CompileMetaKind::StructVariant { object, .. } => {
                write!(fmt, "variant {}", object.item)?;
            }
            CompileMetaKind::Enum { item, .. } => {
                write!(fmt, "enum {}", item)?;
            }
            CompileMetaKind::Function { item, .. } => {
                write!(fmt, "fn {}", item)?;
            }
            CompileMetaKind::Closure { item, .. } => {
                write!(fmt, "closure {}", item)?;
            }
            CompileMetaKind::AsyncBlock { item, .. } => {
                write!(fmt, "async block {}", item)?;
            }
            CompileMetaKind::Macro { item, .. } => {
                write!(fmt, "macro {}", item)?;
            }
        }

        Ok(())
    }
}

/// Compile-time metadata kind about a unit.
#[derive(Debug, Clone)]
pub enum CompileMetaKind {
    /// Metadata about a tuple.
    Tuple {
        /// The value type associated with this meta item.
        type_of: Type,
        /// The underlying tuple.
        tuple: CompileMetaTuple,
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
    /// Metadata about an object.
    Struct {
        /// The value type associated with this meta item.
        type_of: Type,
        /// The underlying object.
        object: CompileMetaStruct,
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
        /// The item of the enum.
        item: Item,
    },
    /// A function declaration.
    Function {
        /// The value type associated with this meta item.
        type_of: Type,
        /// The item of the function declaration.
        item: Item,
    },
    /// A closure.
    Closure {
        /// The value type associated with this meta item.
        type_of: Type,
        /// The item of the closure.
        item: Item,
        /// Sequence of captured variables.
        captures: Arc<Vec<CompileMetaCapture>>,
    },
    /// An async block.
    AsyncBlock {
        /// The span where the async block is declared.
        type_of: Type,
        /// The item of the closure.
        item: Item,
        /// Sequence of captured variables.
        captures: Arc<Vec<CompileMetaCapture>>,
    },
    /// A macro.
    Macro {
        /// The item of the macro.
        item: Item,
    },
}

/// The metadata about a type.
#[derive(Debug, Clone)]
pub struct CompileMetaStruct {
    /// The path to the object.
    pub item: Item,
    /// Fields associated with the type.
    pub fields: Option<HashSet<String>>,
}

/// The metadata about a variant.
#[derive(Debug, Clone)]
pub struct CompileMetaTuple {
    /// The path to the tuple.
    pub item: Item,
    /// The number of arguments the variant takes.
    pub args: usize,
    /// Hash of the constructor function.
    pub hash: Hash,
}
