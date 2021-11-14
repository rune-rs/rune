use crate::ast::Span;
use crate::compile::Meta;
use crate::SourceId;

/// A visitor that will be called for every language item compiled.
pub trait CompileVisitor {
    /// Called when a meta item is registered.
    fn register_meta(&mut self, _meta: &Meta) {}

    /// Mark that we've encountered a specific compile meta at the given span.
    fn visit_meta(&mut self, _source_id: SourceId, _meta: &Meta, _span: Span) {}

    /// Visit a variable use.
    fn visit_variable_use(&mut self, _source_id: SourceId, _var_span: Span, _span: Span) {}

    /// Visit something that is a module.
    fn visit_mod(&mut self, _source_id: SourceId, _span: Span) {}
}

/// A [CompileVisitor] which does nothing.
pub(crate) struct NoopCompileVisitor(());

impl NoopCompileVisitor {
    /// Construct a new noop compile visitor.
    pub(crate) const fn new() -> Self {
        Self(())
    }
}

impl CompileVisitor for NoopCompileVisitor {}
