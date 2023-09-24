use crate::ast::Spanned;
use crate::compile::{Item, Located, MetaError, MetaRef};
use crate::hash::Hash;
use crate::SourceId;

/// A visitor that will be called for every language item compiled.
pub trait CompileVisitor {
    /// Called when a meta item is registered.
    fn register_meta(&mut self, _meta: MetaRef<'_>) -> Result<(), MetaError> {
        Ok(())
    }

    /// Mark that we've resolved a specific compile meta at the given location.
    fn visit_meta(&mut self, _location: &dyn Located, _meta: MetaRef<'_>) -> Result<(), MetaError> {
        Ok(())
    }

    /// Visit a variable use.
    fn visit_variable_use(
        &mut self,
        _source_id: SourceId,
        _var_span: &dyn Spanned,
        _span: &dyn Spanned,
    ) -> Result<(), MetaError> {
        Ok(())
    }

    /// Visit something that is a module.
    fn visit_mod(&mut self, _location: &dyn Located) -> Result<(), MetaError> {
        Ok(())
    }

    /// Visit anterior `///`-style comments, and interior `//!`-style doc
    /// comments for an item.
    ///
    /// This may be called several times for a single item. Each attribute
    /// should eventually be combined for the full doc string.
    ///
    /// This can be called in any order, before or after
    /// [CompileVisitor::visit_meta] for any given item.
    fn visit_doc_comment(
        &mut self,
        _location: &dyn Located,
        _item: &Item,
        _hash: Hash,
        _docstr: &str,
    ) -> Result<(), MetaError> {
        Ok(())
    }

    /// Visit anterior `///`-style comments, and interior `//!`-style doc
    /// comments for a field contained in a struct / enum variant struct.
    ///
    /// This may be called several times for a single field. Each attribute
    /// should eventually be combined for the full doc string.
    fn visit_field_doc_comment(
        &mut self,
        _location: &dyn Located,
        _item: &Item,
        _hash: Hash,
        _field: &str,
        _docstr: &str,
    ) -> Result<(), MetaError> {
        Ok(())
    }
}

/// A [CompileVisitor] which does nothing.
#[cfg(feature = "std")]
pub(crate) struct NoopCompileVisitor(());

#[cfg(feature = "std")]
impl NoopCompileVisitor {
    /// Construct a new noop compile visitor.
    pub(crate) const fn new() -> Self {
        Self(())
    }
}

#[cfg(feature = "std")]
impl CompileVisitor for NoopCompileVisitor {}
