use core::fmt;

use crate::no_std::collections::HashMap;
use crate::no_std::prelude::*;

use crate::ast::{Span, Spanned};
use crate::compile::v1::Assembler;
use crate::compile::{self, Assembly, CompileErrorKind, CompileVisitor, WithSpan};
use crate::hir;
use crate::runtime::Inst;
use crate::SourceId;

/// A locally declared variable, its calculated stack offset and where it was
/// declared in its source file.
#[derive(Debug, Clone, Copy)]
pub struct Var {
    /// Slot offset from the current stack frame.
    pub(crate) offset: usize,
    /// Token assocaited with the variable.
    span: Span,
    /// Variable has been taken at the given position.
    moved_at: Option<Span>,
}

impl Var {
    /// Copy the declared variable.
    pub(crate) fn copy<C>(&self, c: &mut Assembler<'_, '_>, span: &dyn Spanned, comment: C)
    where
        C: fmt::Display,
    {
        c.asm.push_with_comment(
            Inst::Copy {
                offset: self.offset,
            },
            span,
            comment,
        );
    }

    /// Move the declared variable.
    pub(crate) fn do_move<C>(&self, asm: &mut Assembly, span: &dyn Spanned, comment: C)
    where
        C: fmt::Display,
    {
        asm.push_with_comment(
            Inst::Move {
                offset: self.offset,
            },
            span,
            comment,
        );
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Layer<'hir> {
    /// Named variables.
    variables: HashMap<&'hir str, Var>,
    /// The number of variables.
    pub(crate) total: usize,
    /// The number of variables local to this scope.
    pub(crate) local: usize,
}

impl<'hir> Layer<'hir> {
    /// Construct a new locals handlers.
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            total: 0,
            local: 0,
        }
    }

    /// Construct a new child scope.
    fn child(&self) -> Self {
        Self {
            variables: HashMap::new(),
            total: self.total,
            local: 0,
        }
    }
}

/// A guard returned from [push][Scopes::push].
///
/// This should be provided to a subsequent [pop][Scopes::pop] to allow it to be
/// sanity checked.
#[must_use]
pub(crate) struct ScopeGuard(usize);

pub(crate) struct Scopes<'hir> {
    layers: Vec<Layer<'hir>>,
}

impl<'hir> Scopes<'hir> {
    /// Construct a new collection of scopes.
    pub(crate) fn new() -> Self {
        Self {
            layers: vec![Layer::new()],
        }
    }

    /// Get the local with the given name.
    #[tracing::instrument(skip_all, fields(variable, name, source_id))]
    pub(crate) fn get(
        &self,
        visitor: &mut dyn CompileVisitor,
        variable: hir::Variable,
        name: &'hir str,
        source_id: SourceId,
        span: &dyn Spanned,
    ) -> compile::Result<Var> {
        tracing::trace!("get");

        for layer in self.layers.iter().rev() {
            if let Some(var) = layer.variables.get(name) {
                tracing::trace!(?variable, ?var, "getting var");
                visitor.visit_variable_use(source_id, var.span, span);

                if let Some(moved_at) = var.moved_at {
                    return Err(compile::Error::new(
                        span,
                        CompileErrorKind::VariableMoved { moved_at },
                    ));
                }

                return Ok(*var);
            }
        }

        Err(compile::Error::msg(
            span,
            format_args!("Missing variable `{name}` ({variable})"),
        ))
    }

    /// Take the local with the given name.
    #[tracing::instrument(skip_all, fields(variable, name, source_id))]
    pub(crate) fn take(
        &mut self,
        visitor: &mut dyn CompileVisitor,
        variable: hir::Variable,
        name: &'hir str,
        source_id: SourceId,
        span: &dyn Spanned,
    ) -> compile::Result<&Var> {
        tracing::trace!("take");

        for layer in self.layers.iter_mut().rev() {
            if let Some(var) = layer.variables.get_mut(name) {
                tracing::trace!(?variable, ?var, "taking var");
                visitor.visit_variable_use(source_id, var.span, span);

                if let Some(moved_at) = var.moved_at {
                    return Err(compile::Error::new(
                        span,
                        CompileErrorKind::VariableMoved { moved_at },
                    ));
                }

                var.moved_at = Some(span.span());
                return Ok(var);
            }
        }

        Err(compile::Error::msg(
            span,
            format_args!("Missing variable `{name}` to take ({variable})"),
        ))
    }

    /// Construct a new variable.
    #[tracing::instrument(skip_all, fields(variable, name))]
    pub(crate) fn define(
        &mut self,
        #[allow(unused)] variable: hir::Variable,
        name: &'hir str,
        span: &dyn Spanned,
    ) -> compile::Result<usize> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        tracing::trace!(?layer);

        let offset = layer.total;

        let local = Var {
            offset,
            span: span.span(),
            moved_at: None,
        };

        layer.total += 1;
        layer.local += 1;
        layer.variables.insert(name, local);
        Ok(offset)
    }

    /// Declare an anonymous variable.
    #[tracing::instrument(skip_all)]
    pub(crate) fn alloc(&mut self, span: &dyn Spanned) -> compile::Result<usize> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        tracing::trace!(?layer);

        let offset = layer.total;
        layer.total += 1;
        layer.local += 1;
        Ok(offset)
    }

    /// Free a bunch of anonymous slots.
    #[tracing::instrument(skip_all, fields(n))]
    pub(crate) fn free(&mut self, span: &dyn Spanned, n: usize) -> compile::Result<()> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        tracing::trace!(?layer);

        layer.total = layer
            .total
            .checked_sub(n)
            .ok_or("totals out of bounds")
            .with_span(span)?;

        layer.local = layer
            .local
            .checked_sub(n)
            .ok_or("locals out of bounds")
            .with_span(span)?;

        Ok(())
    }

    /// Pop the last scope and compare with the expected length.
    #[tracing::instrument(skip_all, fields(expected))]
    pub(crate) fn pop(
        &mut self,
        expected: ScopeGuard,
        span: &dyn Spanned,
    ) -> compile::Result<Layer<'hir>> {
        let ScopeGuard(expected) = expected;

        if self.layers.len() != expected {
            return Err(compile::Error::msg(
                span,
                format_args!(
                    "Scope guard mismatch, {} (actual) != {} (expected)",
                    self.layers.len(),
                    expected
                ),
            ));
        }

        let Some(layer) = self.layers.pop() else {
            return Err(compile::Error::msg(span, "Missing parent scope"));
        };

        tracing::trace!(?layer, "pop");
        Ok(layer)
    }

    /// Pop the last of the scope.
    pub(crate) fn pop_last(&mut self, span: &dyn Spanned) -> compile::Result<Layer<'hir>> {
        self.pop(ScopeGuard(1), span)
    }

    /// Construct a new child scope and return its guard.
    #[tracing::instrument(skip_all)]
    pub(crate) fn child(&mut self, span: &dyn Spanned) -> compile::Result<ScopeGuard> {
        let Some(layer) = self.layers.last() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        tracing::trace!(?layer);
        Ok(self.push(layer.child()))
    }

    /// Get the total var count of the top scope.
    pub(crate) fn total(&self, span: &dyn Spanned) -> compile::Result<usize> {
        let Some(layer) = self.layers.last() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        Ok(layer.total)
    }

    /// Get the local var count of the top scope.
    pub(crate) fn local(&self, span: &dyn Spanned) -> compile::Result<usize> {
        let Some(layer) = self.layers.last() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        Ok(layer.local)
    }

    /// Push a scope and return an index.
    pub(crate) fn push(&mut self, layer: Layer<'hir>) -> ScopeGuard {
        self.layers.push(layer);
        ScopeGuard(self.layers.len())
    }
}
