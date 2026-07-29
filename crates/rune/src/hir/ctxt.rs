use tracing::instrument_ast;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::ast::{self, Spanned};
use crate::compile::{meta, DynLocation, Error, ErrorKind, ItemId, Result};
use crate::grammar::{Ignore, Node};
use crate::hir;
use crate::query::{GenericsParameters, Query, SecondaryBuildEntry};
use crate::SourceId;

/// How deeply generic arguments are allowed to nest.
///
/// Lowering them recurses, and a few hundred levels was enough to exhaust the
/// stack a test runs on when this was measured, so the bound is well under that
/// while staying far past anything a path is written with.
const MAX_GENERICS_DEPTH: usize = 32;

#[derive(Default, Clone, Copy)]
pub(super) enum Needs {
    #[default]
    Value,
    Type,
}

pub(crate) struct Ctxt<'hir, 'a, 'arena> {
    /// Arena used for allocations.
    pub(super) arena: &'hir hir::arena::Arena,
    /// Expressions lowered so far.
    ///
    /// Expressions refer to their children by [`hir::ExprId`], so this has to
    /// outlive the lowered item and is handed to whichever pass consumes it.
    pub(crate) exprs: hir::Exprs<'hir>,
    pub(crate) q: Query<'a, 'arena>,
    pub(super) source_id: SourceId,
    pub(super) const_eval: bool,
    pub(super) secondary_builds: Option<&'a mut Vec<SecondaryBuildEntry<'hir>>>,
    pub(super) in_template: bool,
    pub(super) needs: Needs,
    /// How deeply the constant expression being lowered is nested.
    ///
    /// Only maintained while `const_eval` is set.
    pub(super) const_depth: usize,
    /// How deeply the expansion being lowered is nested.
    ///
    /// See [`Ctxt::enter_expansion`].
    pub(super) expansion_depth: usize,
    /// How deeply the generic arguments being lowered are nested.
    ///
    /// See [`Ctxt::enter_generics`].
    pub(super) generics_depth: usize,
    pub(super) scopes: hir::Scopes<'hir, 'a>,
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
            exprs: hir::Exprs::new(),
            q,
            source_id,
            const_eval,
            secondary_builds,
            in_template: false,
            needs: Needs::default(),
            const_depth: 0,
            expansion_depth: 0,
            generics_depth: 0,
            scopes,
            statements: Vec::new(),
            pattern_bindings: Vec::new(),
            label: None,
        })
    }

    /// Take the expressions lowered so far, leaving the context empty.
    ///
    /// Called once lowering of an item is complete, so the store can be handed
    /// to the pass which consumes it.
    pub(crate) fn take_exprs(&mut self) -> hir::Exprs<'hir> {
        core::mem::take(&mut self.exprs)
    }

    /// Enter an expansion, bounding how deeply they are allowed to nest.
    ///
    /// An expansion - a macro, a template literal or a format specification -
    /// is lowered from a tree of its own rather than from the one being walked,
    /// so it cannot be parked in the driver's frames and is lowered by
    /// recursing. `max-macro-depth` is what keeps that recursion from
    /// overflowing, so it has to stay well under what the native stack can take
    /// rather than merely being an explicit limit.
    pub(super) fn enter_expansion(&mut self, span: &dyn Spanned) -> Result<()> {
        let max = self.q.options.max_macro_depth;

        if self.expansion_depth >= max {
            return Err(Error::new(
                span,
                ErrorKind::MaxMacroRecursion {
                    depth: self.expansion_depth,
                    max,
                },
            ));
        }

        self.expansion_depth += 1;
        Ok(())
    }

    /// Leave the expansion entered by [`Ctxt::enter_expansion`].
    pub(super) fn leave_expansion(&mut self) {
        self.expansion_depth -= 1;
    }

    /// Enter a level of generic arguments, bounding how deeply they nest.
    ///
    /// The arguments of a path are lowered from a stream of their own rather
    /// than from the tree being walked, so they cannot be parked in the driver's
    /// frames and are lowered by recursing. The bound therefore has to stay well
    /// under what the native stack can take rather than merely being an explicit
    /// limit, which is what [`MAX_GENERICS_DEPTH`] is, and `max-depth` can lower
    /// it but not raise it.
    pub(super) fn enter_generics(&mut self, span: &dyn Spanned) -> Result<()> {
        let max = self.q.options.max_depth.min(MAX_GENERICS_DEPTH);

        if self.generics_depth >= max {
            return Err(Error::new(span, ErrorKind::MaxGenericsDepth { max }));
        }

        self.generics_depth += 1;
        Ok(())
    }

    /// Leave the generic arguments entered by [`Ctxt::enter_generics`].
    pub(super) fn leave_generics(&mut self) {
        self.generics_depth -= 1;
    }

    /// Bound how deeply a constant expression is allowed to nest.
    ///
    /// A constant is evaluated into a [`ConstValue`], which is a recursive
    /// structure - it is built, walked and dropped by recursing over it - so
    /// how deeply a constant nests has to be bounded by something smaller than
    /// the native stack, no matter how much nesting the rest of the compiler
    /// accepts. That bound is the `max-const-depth` option, which `max-depth`
    /// can lower but not raise, since raising it would trade a diagnostic for a
    /// stack overflow.
    ///
    /// [`ConstValue`]: crate::runtime::ConstValue
    pub(super) fn const_nesting(&self, span: &dyn Spanned, depth: usize) -> Result<()> {
        if !self.const_eval {
            return Ok(());
        }

        let max = self
            .q
            .options
            .max_depth
            .min(self.q.options.max_const_depth)
            .min(crate::runtime::MAX_CONST_DEPTH);

        if depth >= max {
            return Err(Error::new(span, ErrorKind::MaxConstDepth { max }));
        }

        Ok(())
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
