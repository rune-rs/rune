use crate::Var;
use runestick::{CompileMeta, Span};

/// A visitor that will be called for every language item compiled.
pub trait CompileVisitor {
    /// Mark that we've encountered a specific compile meta at the given span.
    fn visit_meta(&mut self, _meta: &CompileMeta, _span: Span) {}

    /// Visit a variable use.
    fn visit_variable_use(&mut self, _var: &Var, _span: Span) {}
}

/// A compile visitor that does nothing.
pub struct NoopCompileVisitor(());

impl NoopCompileVisitor {
    /// Construct a new noop compile visitor.
    pub const fn new() -> Self {
        Self(())
    }
}

impl CompileVisitor for NoopCompileVisitor {}
