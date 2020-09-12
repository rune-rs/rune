use crate::collections::HashSet;
use crate::{Hash, Item, Type};
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
pub enum CompileMeta {
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
        /// The value type associated with this meta item.
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

impl CompileMeta {
    /// Get the item of the meta.
    pub fn item(&self) -> &Item {
        match self {
            CompileMeta::Tuple { tuple, .. } => &tuple.item,
            CompileMeta::TupleVariant { tuple, .. } => &tuple.item,
            CompileMeta::Struct { object, .. } => &object.item,
            CompileMeta::StructVariant { object, .. } => &object.item,
            CompileMeta::Enum { item, .. } => item,
            CompileMeta::Function { item, .. } => item,
            CompileMeta::Closure { item, .. } => item,
            CompileMeta::AsyncBlock { item, .. } => item,
            CompileMeta::Macro { item, .. } => item,
        }
    }

    /// Get the value type of the meta item.
    pub fn type_of(&self) -> Option<Type> {
        match self {
            Self::Tuple { type_of, .. } => Some(*type_of),
            Self::TupleVariant { .. } => None,
            Self::Struct { type_of, .. } => Some(*type_of),
            Self::StructVariant { .. } => None,
            Self::Enum { type_of, .. } => Some(*type_of),
            Self::Function { type_of, .. } => Some(*type_of),
            Self::Closure { type_of, .. } => Some(*type_of),
            Self::AsyncBlock { type_of, .. } => Some(*type_of),
            Self::Macro { .. } => None,
        }
    }
}

impl fmt::Display for CompileMeta {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tuple { tuple, .. } => {
                write!(fmt, "struct {}", tuple.item)?;
            }
            Self::TupleVariant { tuple, .. } => {
                write!(fmt, "variant {}", tuple.item)?;
            }
            Self::Struct { object, .. } => {
                write!(fmt, "struct {}", object.item)?;
            }
            Self::StructVariant { object, .. } => {
                write!(fmt, "variant {}", object.item)?;
            }
            Self::Enum { item, .. } => {
                write!(fmt, "enum {}", item)?;
            }
            Self::Function { item, .. } => {
                write!(fmt, "fn {}", item)?;
            }
            Self::Closure { item, .. } => {
                write!(fmt, "closure {}", item)?;
            }
            Self::AsyncBlock { item, .. } => {
                write!(fmt, "async block {}", item)?;
            }
            Self::Macro { item, .. } => {
                write!(fmt, "macro {}", item)?;
            }
        }

        Ok(())
    }
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
