use crate::collections::HashSet;
use crate::context::Item;
use std::fmt;

/// Metadata about an item in the context.
#[derive(Debug, Clone)]
pub enum Meta {
    /// Metadata about a tuple.
    MetaTuple {
        /// The underlying tuple.
        tuple: MetaTuple,
    },
    /// Metadata about a tuple variant.
    MetaTupleVariant {
        /// The item of the enum.
        enum_item: Item,
        /// The underlying tuple.
        tuple: MetaTuple,
    },
    /// Metadata about an object.
    MetaObject {
        /// The underlying object.
        object: MetaObject,
    },
    /// Metadata about a variant object.
    MetaObjectVariant {
        /// The item of the enum.
        enum_item: Item,
        /// The underlying object.
        object: MetaObject,
    },
    /// An enum item.
    MetaEnum {
        /// The item of the enum.
        item: Item,
    },
}

impl fmt::Display for Meta {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MetaTuple { tuple } => {
                write!(fmt, "{item}({args})", item = tuple.item, args = tuple.args)?;
            }
            Self::MetaTupleVariant { tuple, .. } => {
                write!(fmt, "{item}({args})", item = tuple.item, args = tuple.args)?;
            }
            Self::MetaObject { object } => {
                write!(fmt, "{item}", item = object.item)?;
            }
            Self::MetaObjectVariant { object, .. } => {
                write!(fmt, "{item}", item = object.item)?;
            }
            Self::MetaEnum { item, .. } => {
                write!(fmt, "{item}", item = item)?;
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
pub struct MetaObject {
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
}
