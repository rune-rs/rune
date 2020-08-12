use crate::context::Item;
use std::fmt;

/// Metadata about an item in the context.
#[derive(Debug, Clone)]
pub enum Meta {
    /// Metadata about a variant.
    MetaTuple(MetaTuple),
    /// Metadata about a type.
    MetaType(MetaType),
}

impl fmt::Display for Meta {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MetaTuple(tuple) => {
                write!(fmt, "{item}({args})", item = tuple.item, args = tuple.args)?;
            }
            Self::MetaType(ty) => {
                write!(fmt, "{item}", item = ty.item)?;
            }
        }

        Ok(())
    }
}

/// The metadata about a type.
#[derive(Debug, Clone)]
pub struct MetaType {
    /// The path to the type.
    pub item: Item,
}

/// The metadata about a variant.
#[derive(Debug, Clone)]
pub struct MetaTuple {
    /// If the tuple definition is external (native), or internal.
    // TODO: remove once Result's and Option's are typed tuples.
    pub external: bool,
    /// The path to the tuple.
    pub item: Item,
    /// The number of arguments the variant takes.
    pub args: usize,
}
