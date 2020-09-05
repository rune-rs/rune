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

/// Metadata about an item in the context.
#[derive(Debug, Clone)]
pub enum Meta {
    /// Metadata about a tuple.
    MetaTuple {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The underlying tuple.
        tuple: MetaTuple,
    },
    /// Metadata about a tuple variant.
    MetaVariantTuple {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The item of the enum.
        enum_item: Item,
        /// The underlying tuple.
        tuple: MetaTuple,
    },
    /// Metadata about an object.
    MetaStruct {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The underlying object.
        object: MetaStruct,
    },
    /// Metadata about a variant object.
    MetaVariantStruct {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The item of the enum.
        enum_item: Item,
        /// The underlying object.
        object: MetaStruct,
    },
    /// An enum item.
    MetaEnum {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The item of the enum.
        item: Item,
    },
    /// A function declaration.
    MetaFunction {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The item of the function declaration.
        item: Item,
    },
    /// A closure.
    MetaClosure {
        /// The value type associated with this meta item.
        value_type: Type,
        /// The item of the closure.
        item: Item,
        /// Sequence of captured variables.
        captures: Arc<Vec<MetaClosureCapture>>,
    },
    /// An async block.
    MetaAsyncBlock {
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
            Meta::MetaTuple { tuple, .. } => &tuple.item,
            Meta::MetaVariantTuple { tuple, .. } => &tuple.item,
            Meta::MetaStruct { object, .. } => &object.item,
            Meta::MetaVariantStruct { object, .. } => &object.item,
            Meta::MetaEnum { item, .. } => item,
            Meta::MetaFunction { item, .. } => item,
            Meta::MetaClosure { item, .. } => item,
            Meta::MetaAsyncBlock { item, .. } => item,
        }
    }

    /// Get the value type of the meta item.
    pub fn value_type(&self) -> Option<Type> {
        match self {
            Self::MetaTuple { value_type, .. } => Some(*value_type),
            Self::MetaVariantTuple { .. } => None,
            Self::MetaStruct { value_type, .. } => Some(*value_type),
            Self::MetaVariantStruct { .. } => None,
            Self::MetaEnum { value_type, .. } => Some(*value_type),
            Self::MetaFunction { value_type, .. } => Some(*value_type),
            Self::MetaClosure { value_type, .. } => Some(*value_type),
            Self::MetaAsyncBlock { value_type, .. } => Some(*value_type),
        }
    }
}

impl fmt::Display for Meta {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MetaTuple { tuple, .. } => {
                write!(fmt, "struct {}", tuple.item)?;
            }
            Self::MetaVariantTuple { tuple, .. } => {
                write!(fmt, "variant {}", tuple.item)?;
            }
            Self::MetaStruct { object, .. } => {
                write!(fmt, "struct {}", object.item)?;
            }
            Self::MetaVariantStruct { object, .. } => {
                write!(fmt, "variant {}", object.item)?;
            }
            Self::MetaEnum { item, .. } => {
                write!(fmt, "enum {}", item)?;
            }
            Self::MetaFunction { item, .. } => {
                write!(fmt, "fn {}", item)?;
            }
            Self::MetaClosure { item, .. } => {
                write!(fmt, "closure {}", item)?;
            }
            Self::MetaAsyncBlock { item, .. } => {
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
