use crate::collections::HashSet;
use crate::{Hash, Item, Type};
use std::fmt;
use std::sync::Arc;

/// Metadata about a closure.
#[derive(Debug, Clone)]
pub struct MetaClosureCapture {
    /// Identity of the captured variable.
    pub ident: String,
}

/// Compile-time metadata about a unit.
#[derive(Debug, Clone)]
pub enum Meta {
    /// Metadata about a tuple.
    Tuple {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The underlying tuple.
        tuple: MetaTuple,
    },
    /// Metadata about a tuple variant.
    VariantTuple {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The item of the enum.
        enum_item: Item,
        /// The underlying tuple.
        tuple: MetaTuple,
    },
    /// Metadata about an object.
    Struct {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The underlying object.
        object: MetaStruct,
    },
    /// Metadata about a variant object.
    VariantStruct {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The item of the enum.
        enum_item: Item,
        /// The underlying object.
        object: MetaStruct,
    },
    /// An enum item.
    Enum {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The item of the enum.
        item: Item,
    },
    /// A function declaration.
    Function {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The item of the function declaration.
        item: Item,
    },
    /// A closure.
    Closure {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The item of the closure.
        item: Item,
        /// Sequence of captured variables.
        captures: Arc<Vec<MetaClosureCapture>>,
    },
    /// An async block.
    AsyncBlock {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The item of the closure.
        item: Item,
        /// Sequence of captured variables.
        captures: Arc<Vec<MetaClosureCapture>>,
    },
}

impl Meta {
    /// Get the item of the meta.
    pub fn item(&self) -> &Item {
        match self {
            Meta::Tuple { tuple, .. } => &tuple.item,
            Meta::VariantTuple { tuple, .. } => &tuple.item,
            Meta::Struct { object, .. } => &object.item,
            Meta::VariantStruct { object, .. } => &object.item,
            Meta::Enum { item, .. } => item,
            Meta::Function { item, .. } => item,
            Meta::Closure { item, .. } => item,
            Meta::AsyncBlock { item, .. } => item,
        }
    }

    /// Get the value type of the meta item.
    pub fn value_type(&self) -> Option<Type> {
        match self {
            Self::Tuple { value_type, .. } => Some(*value_type),
            Self::VariantTuple { .. } => None,
            Self::Struct { value_type, .. } => Some(*value_type),
            Self::VariantStruct { .. } => None,
            Self::Enum { value_type, .. } => Some(*value_type),
            Self::Function { value_type, .. } => Some(*value_type),
            Self::Closure { value_type, .. } => Some(*value_type),
            Self::AsyncBlock { value_type, .. } => Some(*value_type),
        }
    }
}

impl fmt::Display for Meta {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tuple { tuple, .. } => {
                write!(fmt, "struct {}", tuple.item)?;
            }
            Self::VariantTuple { tuple, .. } => {
                write!(fmt, "variant {}", tuple.item)?;
            }
            Self::Struct { object, .. } => {
                write!(fmt, "struct {}", object.item)?;
            }
            Self::VariantStruct { object, .. } => {
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
        }

        Ok(())
    }
}

/// The metadata about a type.
#[derive(Debug, Clone)]
pub struct MetaExternal {
    /// The path to the type.
    pub item: Item,
}

/// The metadata about a type.
#[derive(Debug, Clone)]
pub struct MetaStruct {
    /// The path to the object.
    pub item: Item,
    /// Fields associated with the type.
    pub fields: Option<HashSet<String>>,
}

/// The metadata about a variant.
#[derive(Debug, Clone)]
pub struct MetaTuple {
    /// The path to the tuple.
    pub item: Item,
    /// The number of arguments the variant takes.
    pub args: usize,
    /// Hash of the constructor function.
    pub hash: Hash,
}
