use crate::internal::commas;
use crate::Assign;
use std::fmt;

/// The definition of an input to a block.
///
/// These are essentially phi nodes, and makes sure that there's a local
/// variable declaration available.
#[derive(Debug, Clone)]
pub struct Phi {
    /// The blocks which defines the variable.
    dependencies: Vec<Assign>,
}

impl Phi {
    /// Construct a new phi node.
    pub(crate) fn new() -> Self {
        Self {
            dependencies: Vec::new(),
        }
    }

    /// Extend with the given iterator.
    pub(crate) fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Assign>,
    {
        self.dependencies.extend(iter);
    }
}

impl fmt::Display for Phi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.dependencies.is_empty() {
            write!(f, "φ(?)")?;
        } else {
            write!(f, "φ({})", commas(&self.dependencies))?;
        }

        Ok(())
    }
}
