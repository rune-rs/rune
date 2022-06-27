use crate::ast::Span;
use crate::compile::{
    ir, Assembly, CompileError, CompileErrorKind, CompileResult, IrBudget, IrCompiler,
    IrInterpreter, Item, ItemMeta, Options, PrivMeta,
};
use crate::hir;
use crate::query::{Named, Query, QueryConstFn, Used};
use crate::runtime::{ConstValue, Inst};
use crate::{Context, Diagnostics, SourceId};

pub(crate) mod assemble;
mod loops;
mod scopes;

pub(crate) use self::loops::{Loop, Loops};
pub(crate) use self::scopes::{Scope, ScopeGuard, Scopes, Var};

/// A needs hint for an expression.
/// This is used to contextually determine what an expression is expected to
/// produce.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Needs {
    Type,
    Value,
    None,
}

impl Needs {
    /// Test if any sort of value is needed.
    pub(crate) fn value(self) -> bool {
        matches!(self, Self::Type | Self::Value)
    }
}

pub(crate) struct Assembler<'a> {
    /// The source id of the source.
    pub(crate) source_id: SourceId,
    /// The context we are compiling for.
    pub(crate) context: &'a Context,
    /// Query system to compile required items.
    pub(crate) q: Query<'a>,
    /// The assembly we are generating.
    pub(crate) asm: &'a mut Assembly,
    /// Scopes defined in the compiler.
    pub(crate) scopes: Scopes,
    /// Context for which to emit warnings.
    pub(crate) contexts: Vec<Span>,
    /// The nesting of loop we are currently in.
    pub(crate) loops: Loops,
    /// Enabled optimizations.
    pub(crate) options: &'a Options,
    /// Compilation warnings.
    pub(crate) diagnostics: &'a mut Diagnostics,
}

impl<'a> Assembler<'a> {
    /// Access the meta for the given language item.
    pub fn try_lookup_meta(
        &mut self,
        spanned: Span,
        item: &Item,
    ) -> CompileResult<Option<PrivMeta>> {
        tracing::trace!("lookup meta: {:?}", item);

        if let Some(meta) = self.q.query_meta(spanned, item, Default::default())? {
            tracing::trace!("found in query: {:?}", meta);
            self.q
                .visitor
                .visit_meta(self.source_id, meta.info_ref(), spanned);
            return Ok(Some(meta));
        }

        if let Some(meta) = self.context.lookup_meta(item) {
            tracing::trace!("found in context: {:?}", meta);
            self.q
                .visitor
                .visit_meta(self.source_id, meta.info_ref(), spanned);
            return Ok(Some(meta));
        }

        Ok(None)
    }

    /// Access the meta for the given language item.
    pub fn lookup_meta(&mut self, spanned: Span, item: &Item) -> CompileResult<PrivMeta> {
        if let Some(meta) = self.try_lookup_meta(spanned, item)? {
            return Ok(meta);
        }

        Err(CompileError::new(
            spanned,
            CompileErrorKind::MissingItem { item: item.clone() },
        ))
    }

    /// Pop locals by simply popping them.
    pub(crate) fn locals_pop(&mut self, total_var_count: usize, span: Span) {
        match total_var_count {
            0 => (),
            1 => {
                self.asm.push(Inst::Pop, span);
            }
            count => {
                self.asm.push(Inst::PopN { count }, span);
            }
        }
    }

    /// Clean up local variables by preserving the value that is on top and
    /// popping the rest.
    ///
    /// The clean operation will preserve the value that is on top of the stack,
    /// and pop the values under it.
    pub(crate) fn locals_clean(&mut self, total_var_count: usize, span: Span) {
        match total_var_count {
            0 => (),
            count => {
                self.asm.push(Inst::Clean { count }, span);
            }
        }
    }

    /// Convert an [ast::Path] into a [Named] item.
    pub(crate) fn convert_path<'hir>(
        &mut self,
        path: &'hir hir::Path<'hir>,
    ) -> CompileResult<Named<'hir>> {
        self.q.convert_path(self.context, path)
    }

    /// Clean the last scope.
    pub(crate) fn clean_last_scope(
        &mut self,
        span: Span,
        expected: ScopeGuard,
        needs: Needs,
    ) -> CompileResult<()> {
        let scope = self.scopes.pop(expected, span)?;

        if needs.value() {
            self.locals_clean(scope.local_var_count, span);
        } else {
            self.locals_pop(scope.local_var_count, span);
        }

        Ok(())
    }

    /// Get the latest relevant warning context.
    pub(crate) fn context(&self) -> Option<Span> {
        self.contexts.last().copied()
    }

    /// Calling a constant function by id and return the resuling value.
    pub(crate) fn call_const_fn(
        &mut self,
        span: Span,
        meta: &PrivMeta,
        from: &ItemMeta,
        query_const_fn: &QueryConstFn,
        args: &[hir::Expr<'_>],
    ) -> Result<ConstValue, CompileError> {
        if query_const_fn.ir_fn.args.len() != args.len() {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedArgumentCount {
                    meta: meta.info(),
                    expected: query_const_fn.ir_fn.args.len(),
                    actual: args.len(),
                },
            ));
        }

        let mut compiler = IrCompiler {
            source_id: self.source_id,
            q: self.q.borrow(),
        };

        let mut compiled = Vec::new();

        // TODO: precompile these and fetch using opaque id?
        for (hir, name) in args.iter().zip(&query_const_fn.ir_fn.args) {
            compiled.push((ir::compile::expr(hir, &mut compiler)?, name));
        }

        let mut interpreter = IrInterpreter {
            budget: IrBudget::new(1_000_000),
            scopes: Default::default(),
            module: &from.module,
            item: &from.item,
            q: self.q.borrow(),
        };

        for (ir, name) in compiled {
            let value = interpreter.eval_value(&ir, Used::Used)?;
            interpreter.scopes.decl(name, value, span)?;
        }

        interpreter.module = &query_const_fn.item.module;
        interpreter.item = &query_const_fn.item.item;
        let value = interpreter.eval_value(&query_const_fn.ir_fn.ir, Used::Used)?;
        Ok(value.into_const(span)?)
    }
}
