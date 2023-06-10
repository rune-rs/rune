use crate::ast::Spanned;
use crate::no_std::prelude::*;

use crate::ast::Span;
use crate::compile::ir;
use crate::compile::meta;
use crate::compile::{
    self, Assembly, CompileErrorKind, DynLocation, IrBudget, IrCompiler, IrInterpreter, ItemId,
    ItemMeta, Options, WithSpan,
};
use crate::hir;
use crate::query::{ConstFn, Query, Used};
use crate::runtime::{ConstValue, Inst};
use crate::{Hash, SourceId};

pub(crate) mod assemble;
mod loops;
mod scopes;

pub(crate) use self::loops::{Loop, Loops};
pub(crate) use self::scopes::{Layer, ScopeGuard, Scopes, Var};

/// Generic parameters.
#[derive(Default)]
pub(crate) struct GenericsParameters {
    pub(crate) trailing: usize,
    pub(crate) parameters: [Option<Hash>; 2],
}

impl GenericsParameters {
    pub(crate) fn is_empty(&self) -> bool {
        self.parameters.iter().all(|p| p.is_none())
    }

    pub(crate) fn as_boxed(&self) -> Box<[Option<Hash>]> {
        self.parameters.iter().copied().collect()
    }
}

impl AsRef<GenericsParameters> for GenericsParameters {
    #[inline]
    fn as_ref(&self) -> &GenericsParameters {
        self
    }
}

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

pub(crate) struct Assembler<'a, 'hir> {
    /// The source id of the source.
    pub(crate) source_id: SourceId,
    /// Query system to compile required items.
    pub(crate) q: Query<'a>,
    /// The assembly we are generating.
    pub(crate) asm: &'a mut Assembly,
    /// Scopes defined in the compiler.
    pub(crate) scopes: Scopes<'hir>,
    /// Context for which to emit warnings.
    pub(crate) contexts: Vec<Span>,
    /// The nesting of loop we are currently in.
    pub(crate) loops: Loops,
    /// Enabled optimizations.
    pub(crate) options: &'a Options,
}

impl<'a, 'hir> Assembler<'a, 'hir> {
    /// Access the meta for the given language item.
    pub fn lookup_meta(
        &mut self,
        span: &dyn Spanned,
        item: ItemId,
        parameters: impl AsRef<GenericsParameters>,
    ) -> compile::Result<meta::Meta> {
        self.q
            .lookup_meta(&DynLocation::new(self.source_id, span), item, parameters)
    }

    /// Pop locals by simply popping them.
    pub(crate) fn locals_pop(&mut self, total_var_count: usize, span: &dyn Spanned) {
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
    pub(crate) fn locals_clean(&mut self, total_var_count: usize, span: &dyn Spanned) {
        match total_var_count {
            0 => (),
            count => {
                self.asm.push(Inst::Clean { count }, span);
            }
        }
    }

    /// Clean the last scope.
    pub(crate) fn clean_last_scope(
        &mut self,
        span: &dyn Spanned,
        expected: ScopeGuard,
        needs: Needs,
    ) -> compile::Result<()> {
        let scope = self.scopes.pop(expected, span)?;

        if needs.value() {
            self.locals_clean(scope.local, span);
        } else {
            self.locals_pop(scope.local, span);
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
        span: &dyn Spanned,
        from: &ItemMeta,
        query_const_fn: &ConstFn,
        args: &[hir::Expr<'_>],
    ) -> compile::Result<ConstValue> {
        if query_const_fn.ir_fn.args.len() != args.len() {
            return Err(compile::Error::new(
                span,
                CompileErrorKind::UnsupportedArgumentCount {
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
            compiled.push((ir::compiler::expr(hir, &mut compiler)?, name));
        }

        let mut interpreter = IrInterpreter {
            budget: IrBudget::new(1_000_000),
            scopes: Default::default(),
            module: from.module,
            item: from.item,
            q: self.q.borrow(),
        };

        for (ir, name) in compiled {
            let value = interpreter.eval_value(&ir, Used::Used)?;
            interpreter.scopes.decl(name, value).with_span(span)?;
        }

        interpreter.module = query_const_fn.item_meta.module;
        interpreter.item = query_const_fn.item_meta.item;
        let value = interpreter.eval_value(&query_const_fn.ir_fn.ir, Used::Used)?;
        value.into_const(span)
    }
}
