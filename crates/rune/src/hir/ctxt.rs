use tracing::instrument_ast;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::ast::{self, Spanned};
use crate::compile::{meta, DynLocation, Error, ItemId, Result};
use crate::grammar::{Ignore, Node};
use crate::hir;
use crate::query::{GenericsParameters, Query, SecondaryBuildEntry};
use crate::SourceId;

#[derive(Default, Clone, Copy)]
pub(super) enum Needs {
    #[default]
    Value,
    Type,
}

pub(crate) struct Ctxt<'hir, 'a, 'arena> {
    /// Arena used for allocations.
    pub(super) arena: &'hir hir::arena::Arena,
    pub(crate) q: Query<'a, 'arena>,
    pub(super) source_id: SourceId,
    pub(super) const_eval: bool,
    pub(super) secondary_builds: Option<&'a mut Vec<SecondaryBuildEntry<'hir>>>,
    pub(super) in_template: bool,
    pub(super) in_path: bool,
    pub(super) needs: Needs,
    pub(super) scopes: hir::Scopes<'hir, 'a>,
    pub(super) statement_buffer: Vec<hir::Stmt<'hir>>,
    pub(super) statements: Vec<hir::Stmt<'hir>>,
    pub(super) pattern_bindings: Vec<hir::Variable>,
    pub(super) label: Option<ast::Label>,
}

impl<'hir, 'a, 'arena> Ctxt<'hir, 'a, 'arena> {
    /// Construct a new context for used when constants are built separately
    /// through the query system.
    pub(crate) fn with_query(
        arena: &'hir hir::arena::Arena,
        q: Query<'a, 'arena>,
        source_id: SourceId,
        secondary_builds: &'a mut Vec<SecondaryBuildEntry<'hir>>,
    ) -> alloc::Result<Self> {
        Self::inner(arena, q, source_id, false, Some(secondary_builds))
    }

    /// Construct a new context used in a constant context where the resulting
    /// expression is expected to be converted into a constant.
    pub(crate) fn with_const(
        arena: &'hir hir::arena::Arena,
        q: Query<'a, 'arena>,
        source_id: SourceId,
    ) -> alloc::Result<Self> {
        Self::inner(arena, q, source_id, true, None)
    }

    fn inner(
        arena: &'hir hir::arena::Arena,
        q: Query<'a, 'arena>,
        source_id: SourceId,
        const_eval: bool,
        secondary_builds: Option<&'a mut Vec<SecondaryBuildEntry<'hir>>>,
    ) -> alloc::Result<Self> {
        let scopes = hir::Scopes::new(q.gen)?;

        Ok(Self {
            arena,
            q,
            source_id,
            const_eval,
            secondary_builds,
            in_template: false,
            in_path: false,
            needs: Needs::default(),
            scopes,
            statement_buffer: Vec::new(),
            statements: Vec::new(),
            pattern_bindings: Vec::new(),
            label: None,
        })
    }

    #[instrument_ast(span = ast)]
    pub(super) fn try_lookup_meta(
        &mut self,
        span: &dyn Spanned,
        item: ItemId,
        parameters: &GenericsParameters,
    ) -> Result<Option<meta::Meta>> {
        self.q
            .try_lookup_meta(&DynLocation::new(self.source_id, span), item, parameters)
    }

    #[instrument_ast(span = ast)]
    pub(super) fn lookup_meta(
        &mut self,
        span: &dyn Spanned,
        item: ItemId,
        parameters: impl AsRef<GenericsParameters>,
    ) -> Result<meta::Meta> {
        self.q
            .lookup_meta(&DynLocation::new(self.source_id, span), item, parameters)
    }
}

impl<'a> Ignore<'a> for Ctxt<'_, '_, '_> {
    fn ignore(&mut self, _: Node<'a>) -> Result<()> {
        Ok(())
    }

    fn error(&mut self, error: Error) -> alloc::Result<()> {
        self.q.diagnostics.error(self.source_id, error)
    }
}
