//! A simple label used to jump to a code location.

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
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", self.name, self.id)
    }
}
