use crate::collections::HashSet;
use runestick::{Hash, Item, Type};
use std::fmt;
use std::sync::Arc;

/// Metadata about a closure.
#[derive(Debug, Clone)]
pub(crate) struct MetaClosureCapture {
    /// Identity of the captured variable.
    pub(crate) ident: String,
}

/// Metadata about an item in the context.
#[derive(Debug, Clone)]
pub(crate) enum Meta {
    /// Metadata about a tuple.
    MetaTuple {
        /// The underlying tuple.
        tuple: MetaTuple,
    },
    /// Metadata about a tuple variant.
    MetaVariantTuple {
        /// The item of the enum.
        enum_item: Item,
        /// The underlying tuple.
        tuple: MetaTuple,
    },
    /// Metadata about an object.
    MetaStruct {
        /// The underlying object.
        object: MetaStruct,
    },
    /// Metadata about a variant object.
    MetaVariantStruct {
        /// The item of the enum.
        enum_item: Item,
        /// The underlying object.
        object: MetaStruct,
    },
    /// An enum item.
    MetaEnum {
        /// The item of the enum.
        item: Item,
    },
    /// A function declaration.
    MetaFunction {
        /// The item of the function declaration.
        item: Item,
    },
    /// A closure.
    MetaClosure {
        /// The item of the closure.
        item: Item,
        /// Sequence of captured variables.
        captures: Arc<Vec<MetaClosureCapture>>,
    },
}

impl Meta {
    pub(crate) fn from_rune(meta: runestick::Meta) -> Option<Self> {
        Some(match meta {
            runestick::Meta::MetaTuple { tuple} => Self::MetaTuple { tuple: MetaTuple::from_rune(tuple) },
            _ => return None,
        })
    }

    /// Get the item of the meta.
    pub(crate) fn item(&self) -> &Item {
        match self {
            Self::MetaTuple { tuple } => &tuple.item,
            Self::MetaVariantTuple { tuple, .. } => &tuple.item,
            Self::MetaStruct { object } => &object.item,
            Self::MetaVariantStruct { object, .. } => &object.item,
            Self::MetaEnum { item } => item,
            Self::MetaFunction { item } => item,
            Self::MetaClosure { item, .. } => item,
        }
    }
}

impl fmt::Display for Meta {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MetaTuple { tuple } => {
                write!(fmt, "{}({})", tuple.item, tuple.args)?;
            }
            Self::MetaVariantTuple { tuple, .. } => {
                write!(fmt, "{}({})", tuple.item, tuple.args)?;
            }
            Self::MetaStruct { object } => {
                write!(fmt, "{}", object.item)?;
            }
            Self::MetaVariantStruct { object, .. } => {
                write!(fmt, "{}", object.item)?;
            }
            Self::MetaEnum { item, .. } => {
                write!(fmt, "{}", item)?;
            }
            Self::MetaFunction { item, .. } => {
                write!(fmt, "fn {}", item)?;
            }
            Self::MetaClosure { item, .. } => {
                write!(fmt, "closure {}", item)?;
            }
        }

        Ok(())
    }
}

/// The metadata about a type.
#[derive(Debug, Clone)]
pub(crate) struct MetaExternal {
    /// The path to the type.
    pub(crate) item: Item,
}

/// The metadata about a type.
#[derive(Debug, Clone)]
pub(crate) struct MetaStruct {
    /// The path to the object.
    pub(crate) item: Item,
    /// Fields associated with the type.
    pub(crate) fields: Option<HashSet<String>>,
}

/// The metadata about a variant.
#[derive(Debug, Clone)]
pub(crate) struct MetaTuple {
    /// The path to the tuple.
    pub(crate) item: Item,
    /// The number of arguments the variant takes.
    pub(crate) args: usize,
}

impl MetaTuple {
    /// Convert from a rune meta tuple.
    fn from_rune(tuple: runestick::MetaTuple) -> Self {
        Self {
            item: tuple.item,
            args: tuple.args,
        }
    }
}
