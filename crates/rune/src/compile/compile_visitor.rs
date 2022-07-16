use crate::ast::Span;
use crate::compile::{Item, Location, MetaRef};
use crate::SourceId;

/// A visitor that will be called for every language item compiled.
pub trait CompileVisitor {
    /// Called when a meta item is registered.
    fn register_meta(&mut self, _meta: MetaRef<'_>) {}

    /// Mark that we've encountered a specific compile meta at the given span.
    fn visit_meta(&mut self, _source_id: SourceId, _meta: MetaRef<'_>, _span: Span) {}

    /// Visit a variable use.
    fn visit_variable_use(&mut self, _source_id: SourceId, _var_span: Span, _span: Span) {}

    /// Visit something that is a module.
    fn visit_mod(&mut self, _source_id: SourceId, _span: Span) {}

    /// Visit anterior `///`-style comments, and interior `//!`-style doc
    /// comments for an item.
    ///
    /// This may be called several times for a single item. Each attribute
    /// should eventually be combined for the full doc string.
    ///
    /// This can be called in any order, before or after
    /// [CompileVisitor::visit_meta] for any given item.
    fn visit_doc_comment(&mut self, _location: Location, _item: &Item, _docstr: &str) {}

    /// Visit anterior `///`-style comments, and interior `//!`-style doc
    /// comments for a field contained in a struct / enum variant struct.
    ///
    /// This may be called several times for a single field. Each attribute
    /// should eventually be combined for the full doc string.
    fn visit_field_doc_comment(
        &mut self,
        _location: Location,
        _item: &Item,
        _field: &str,
        _docstr: &str,
    ) {
    }
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
