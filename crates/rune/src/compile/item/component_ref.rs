use core::fmt;

use serde::{Deserialize, Serialize};

use crate::compile::Component;

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

impl<'a> ComponentRef<'a> {
    /// Get the component as a string.
    pub(crate) fn as_str(&self) -> Option<&'a str> {
        match self {
            ComponentRef::Str(string) => Some(string),
            _ => None,
        }
    }

    /// Get the identifier of the component if it is an identifier component.
    pub fn id(self) -> Option<usize> {
        match self {
            Self::Id(n) => Some(n),
            _ => None,
        }
    }

    /// Coerce this [ComponentRef] into an owned [Component].
    pub fn to_owned(&self) -> Component {
        match *self {
            ComponentRef::Crate(s) => Component::Crate(s.into()),
            ComponentRef::Str(s) => Component::Str(s.into()),
            ComponentRef::Id(id) => Component::Id(id),
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
