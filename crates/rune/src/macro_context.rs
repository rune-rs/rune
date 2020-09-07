//! Context for a macro.

use runestick::Source;
use std::sync::Arc;

/// Context for a running macro.
pub struct MacroContext {
    source: Arc<Source>,
}

impl MacroContext {
    /// Construct a new macro context.
    pub fn new(source: Arc<Source>) -> Self {
        Self { source }
    }

    /// Access the current source of the macro context.
    pub fn source(&self) -> &Source {
        &*self.source
    }
}
