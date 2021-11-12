use crate::Item;
use std::fmt;

/// Information on the visibility of an item.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Visibility {
    /// Inherited, or private visibility.
    Inherited,
    /// Something that is publicly visible `pub`.
    Public,
    /// Something that is only visible in the current crate `pub(crate)`.
    Crate,
    /// Visible in the parent crate.
    Super,
    /// Only visible in the same crate.
    SelfValue,
}

impl Visibility {
    /// Test if visibility is public.
    pub(crate) fn is_public(self) -> bool {
        matches!(self, Self::Public)
    }

    /// Check if `from` can access `to` with the current visibility.
    pub(crate) fn is_visible(self, from: &Item, to: &Item) -> bool {
        match self {
            Visibility::Inherited | Visibility::SelfValue => from.is_super_of(to, 1),
            Visibility::Super => from.is_super_of(to, 2),
            Visibility::Public => true,
            Visibility::Crate => true,
        }
    }

    /// Check if `from` can access `to` with the current visibility.
    pub(crate) fn is_visible_inside(self, from: &Item, to: &Item) -> bool {
        match self {
            Visibility::Inherited | Visibility::SelfValue => from == to,
            Visibility::Super => from.is_super_of(to, 1),
            Visibility::Public => true,
            Visibility::Crate => true,
        }
    }
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Inherited
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Inherited => write!(f, "private")?,
            Visibility::Public => write!(f, "pub")?,
            Visibility::Crate => write!(f, "pub(crate)")?,
            Visibility::Super => write!(f, "pub(super)")?,
            Visibility::SelfValue => write!(f, "pub(self)")?,
        }

        Ok(())
    }
}
