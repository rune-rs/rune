use core::fmt;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap};
use crate::ast::Spanned;
use crate::compile::v1::Ctxt;
use crate::compile::{self, Assembly, ErrorKind, WithSpan};
use crate::hir;
use crate::query::Query;
use crate::runtime::{Inst, InstAddress, Output};
use crate::SourceId;

/// A locally declared variable, its calculated stack offset and where it was
/// declared in its source file.
#[derive(TryClone, Clone, Copy)]
#[try_clone(copy)]
pub struct Var<'hir> {
    /// Offset from the current stack frame.
    pub(crate) offset: usize,
    /// The name of the variable.
    name: hir::Name<'hir>,
    /// Token assocaited with the variable.
    span: &'hir dyn Spanned,
    /// Variable has been taken at the given position.
    moved_at: Option<&'hir dyn Spanned>,
}

impl<'hir> fmt::Debug for Var<'hir> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Var")
            .field("offset", &self.offset)
            .field("name", &self.name)
            .field("span", &self.span.span())
            .field("moved_at", &self.moved_at.map(|s| s.span()))
            .finish()
    }
}

impl<'hir> fmt::Display for Var<'hir> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

impl<'hir> Var<'hir> {
    /// Copy the declared variable.
    pub(crate) fn copy(
        &self,
        cx: &mut Ctxt<'_, '_, '_>,
        span: &dyn Spanned,
        comment: Option<&dyn fmt::Display>,
        out: Output,
    ) -> compile::Result<()> {
        cx.asm.push_with_comment(
            Inst::Copy {
                addr: InstAddress::new(self.offset),
                out,
            },
            span,
            &format_args!("var `{}`{}", self.name, Append("; ", comment)),
        )
    }

    /// Move the declared variable.
    pub(crate) fn do_move(
        &self,
        asm: &mut Assembly,
        span: &dyn Spanned,
        comment: Option<&dyn fmt::Display>,
        out: Output,
    ) -> compile::Result<()> {
        asm.push_with_comment(
            Inst::Move {
                addr: InstAddress::new(self.offset),
                out,
            },
            span,
            &format_args!("var `{}`{}", self.name, Append("; ", comment)),
        )
    }
}

struct Append<P, T>(P, T);

impl<P, T> fmt::Display for Append<P, T>
where
    P: fmt::Display,
    T: Copy + IntoIterator,
    T::Item: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for item in self.1 {
            self.0.fmt(f)?;
            item.fmt(f)?;
        }

        Ok(())
    }
}

#[derive(Debug, TryClone)]
pub(crate) struct Layer<'hir> {
    /// Named variables.
    variables: HashMap<hir::Name<'hir>, Var<'hir>>,
    /// The number of variables.
    pub(crate) size: usize,
}

impl<'hir> Layer<'hir> {
    /// Construct a new locals handlers.
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            size: 0,
        }
    }

    /// Construct a new child scope.
    fn child(&self) -> Self {
        Self {
            variables: HashMap::new(),
            size: self.size,
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
    source_id: SourceId,
    size: usize,
}

impl<'hir> Scopes<'hir> {
    /// Get the maximum total number of variables used in a function.
    /// Effectively the required stack size.
    pub(crate) fn size(&self) -> usize {
        self.size
    }

    /// Construct a new collection of scopes.
    pub(crate) fn new(source_id: SourceId) -> alloc::Result<Self> {
        Ok(Self {
            layers: try_vec![Layer::new()],
            source_id,
            size: 0,
        })
    }

    /// Get the local with the given name.
    #[tracing::instrument(skip_all, fields(variable, name, source_id))]
    pub(crate) fn get(
        &self,
        q: &mut Query<'_, '_>,
        name: hir::Name<'hir>,
        span: &'hir dyn Spanned,
    ) -> compile::Result<Var<'hir>> {
        tracing::trace!("get");

        for layer in self.layers.iter().rev() {
            if let Some(var) = layer.variables.get(&name) {
                tracing::trace!(?var, "getting var");
                q.visitor
                    .visit_variable_use(self.source_id, var.span, span)
                    .with_span(span)?;

                if let Some(_moved_at) = var.moved_at {
                    return Err(compile::Error::new(
                        span,
                        ErrorKind::VariableMoved {
                            #[cfg(feature = "emit")]
                            moved_at: _moved_at.span(),
                        },
                    ));
                }

                return Ok(*var);
            }
        }

        Err(compile::Error::msg(
            span,
            try_format!("Missing variable `{name}`"),
        ))
    }

    /// Take the local with the given name.
    #[tracing::instrument(skip_all, fields(variable, name, source_id))]
    pub(crate) fn take(
        &mut self,
        q: &mut Query<'_, '_>,
        name: hir::Name<'hir>,
        span: &'hir dyn Spanned,
    ) -> compile::Result<&Var> {
        tracing::trace!("take");

        for layer in self.layers.iter_mut().rev() {
            if let Some(var) = layer.variables.get_mut(&name) {
                tracing::trace!(?var, "taking var");
                q.visitor
                    .visit_variable_use(self.source_id, var.span, span)
                    .with_span(span)?;

                if let Some(_moved_at) = var.moved_at {
                    return Err(compile::Error::new(
                        span,
                        ErrorKind::VariableMoved {
                            #[cfg(feature = "emit")]
                            moved_at: _moved_at.span(),
                        },
                    ));
                }

                var.moved_at = Some(span);
                return Ok(var);
            }
        }

        Err(compile::Error::msg(
            span,
            try_format!("Missing variable `{name}` to take"),
        ))
    }

    /// Construct a new variable.
    #[tracing::instrument(skip_all, fields(variable, name))]
    pub(crate) fn define(
        &mut self,
        name: hir::Name<'hir>,
        span: &'hir dyn Spanned,
    ) -> compile::Result<usize> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        tracing::trace!(?layer);

        let offset = layer.size;

        let var = Var {
            offset,
            name,
            span,
            moved_at: None,
        };

        layer.variables.try_insert(name, var)?;
        layer.size += 1;
        self.size = self.size.max(layer.size);
        Ok(offset)
    }

    /// Declare an anonymous variable.
    #[tracing::instrument(skip_all)]
    pub(crate) fn alloc(&mut self, span: &dyn Spanned) -> compile::Result<InstAddress> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        tracing::trace!(?layer);

        let offset = layer.size;
        layer.size += 1;
        self.size = self.size.max(layer.size);
        Ok(InstAddress::new(offset))
    }

    /// Free a bunch of anonymous slots.
    #[tracing::instrument(skip_all, fields(n))]
    pub(crate) fn free(&mut self, span: &dyn Spanned, n: usize) -> compile::Result<()> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        tracing::trace!(?layer);

        layer.size = layer
            .size
            .checked_sub(n)
            .ok_or("totals out of bounds")
            .with_span(span)?;

        Ok(())
    }

    /// Pop the last scope and compare with the expected length.
    #[tracing::instrument(skip_all, fields(expected))]
    pub(crate) fn pop(
        &mut self,
        span: &dyn Spanned,
        expected: ScopeGuard,
    ) -> compile::Result<Layer<'hir>> {
        let ScopeGuard(expected) = expected;

        if self.layers.len() != expected {
            return Err(compile::Error::msg(
                span,
                try_format!(
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
        self.pop(span, ScopeGuard(1))
    }

    /// Construct a new child scope and return its guard.
    #[tracing::instrument(skip_all)]
    pub(crate) fn child(&mut self, span: &dyn Spanned) -> compile::Result<ScopeGuard> {
        let Some(layer) = self.layers.last() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        tracing::trace!(?layer);
        Ok(self.push(layer.child())?)
    }

    /// Get the total var count of the top scope.
    pub(crate) fn total(&self, span: &dyn Spanned) -> compile::Result<usize> {
        let Some(layer) = self.layers.last() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        Ok(layer.size)
    }

    /// Push a scope and return an index.
    pub(crate) fn push(&mut self, layer: Layer<'hir>) -> alloc::Result<ScopeGuard> {
        self.layers.try_push(layer)?;
        Ok(ScopeGuard(self.layers.len()))
    }
}
