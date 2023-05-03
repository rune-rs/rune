use core::fmt;

use crate::no_std::prelude::*;

use serde::{Deserialize, Serialize};

use crate::compile::ComponentRef;

/// The component of an item.
///
/// All indexes refer to sibling indexes. So two sibling id components could
/// have the indexes `1` and `2` respectively.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Component {
    /// A crate component.
    Crate(Box<str>),
    /// A regular string component.
    Str(Box<str>),
    /// A nested anonymous part with an identifier.
    Id(usize),
}

impl Component {
    /// Get the identifier of the component.
    pub fn id(&self) -> Option<usize> {
        match self {
            Self::Id(n) => Some(*n),
            _ => None,
        }
    }

    /// Convert into component reference.
    pub fn as_component_ref(&self) -> ComponentRef<'_> {
        match self {
            Self::Crate(s) => ComponentRef::Crate(s),
            Self::Str(s) => ComponentRef::Str(s),
            Self::Id(n) => ComponentRef::Id(*n),
        }
    }
}

impl fmt::Display for Component {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Crate(s) => write!(fmt, "::{}", s),
            Self::Str(s) => write!(fmt, "{}", s),
            Self::Id(n) => write!(fmt, "${}", n),
        }
    }
}
