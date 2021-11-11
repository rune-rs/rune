//! A simple label used to jump to a code location.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

/// A label that can be jumped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label {
    name: &'static str,
    id: usize,
}

impl Label {
    /// Construct a new label.
    pub fn new(name: &'static str, id: usize) -> Self {
        Self { name, id }
    }

    /// Convert into owned label.
    pub fn into_owned(self) -> DebugLabel {
        DebugLabel {
            name: self.name.into(),
            id: self.id,
        }
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", self.name, self.id)
    }
}

/// A label that can be jumped to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugLabel {
    /// The name of the label.
    name: Cow<'static, str>,
    /// The id of the label.
    id: usize,
}

impl fmt::Display for DebugLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", self.name, self.id)
    }
}
