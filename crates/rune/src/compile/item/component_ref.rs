use core::fmt;

use serde::{Deserialize, Serialize};

/// A reference to a component of an item.
///
/// All indexes refer to sibling indexes. So two sibling id components could
/// have the indexes `1` and `2` respectively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ComponentRef<'a> {
    /// A crate string component.
    Crate(&'a str),
    /// A regular string component.
    Str(&'a str),
    /// A nested anonymous part with an identifier.
    Id(usize),
}

impl ComponentRef<'_> {
    /// Get the identifier of the component if it is an identifier component.
    pub fn id(self) -> Option<usize> {
        match self {
            Self::Id(n) => Some(n),
            _ => None,
        }
    }
}

impl fmt::Display for ComponentRef<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Crate(s) => write!(fmt, "::{}", s),
            Self::Str(s) => write!(fmt, "{}", s),
            Self::Id(n) => write!(fmt, "${}", n),
        }
    }
}
