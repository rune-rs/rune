use core::fmt;
use core::iter;
use core::slice;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap, HashSet};
use crate::ast::Spanned;
use crate::compile::v1::Ctxt;
use crate::compile::{self, Assembly, ErrorKind, WithSpan};
use crate::hir;
use crate::query::Query;
use crate::runtime::{Inst, InstAddress, Output};
use crate::SourceId;

use super::Slab;

/// A locally declared variable, its calculated stack offset and where it was
/// declared in its source file.
#[derive(TryClone, Clone, Copy)]
#[try_clone(copy)]
pub struct Var<'hir> {
    /// Offset from the current stack frame.
    pub(crate) addr: InstAddress,
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
            .field("addr", &self.addr)
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
                addr: self.addr,
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
                addr: self.addr,
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
    names: HashMap<hir::Name<'hir>, Var<'hir>>,
    /// Slots owned by this layer.
    owned: HashSet<usize>,
}

impl<'hir> Layer<'hir> {
    /// Construct a new locals handlers.
    fn new() -> Self {
        Self {
            names: HashMap::new(),
            owned: HashSet::new(),
        }
    }

    /// Construct a new child scope.
    fn child(&self) -> Self {
        Self {
            names: HashMap::new(),
            owned: HashSet::new(),
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
    slots: Slab,
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
            slots: Slab::new(),
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
            if let Some(var) = layer.names.get(&name) {
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
            if let Some(var) = layer.names.get_mut(&name) {
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
        addr: InstAddress,
    ) -> compile::Result<()> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        tracing::trace!(?layer);

        let var = Var {
            addr,
            name,
            span,
            moved_at: None,
        };

        layer.names.try_insert(name, var).with_span(span)?;
        layer.owned.try_insert(addr.offset()).with_span(span)?;
        Ok(())
    }

    /// Declare an anonymous variable.
    #[tracing::instrument(skip_all)]
    pub(crate) fn alloc(&mut self, span: &dyn Spanned) -> compile::Result<InstAddress> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        let Some(offset) = self.slots.insert()? else {
            return Err(compile::Error::msg(span, "Ran out of slots"));
        };

        tracing::trace!(?layer);

        layer.owned.try_insert(offset).with_span(span)?;
        self.size = self.size.max(self.slots.len());
        Ok(InstAddress::new(offset))
    }

    /// Peek the next address.
    #[tracing::instrument(skip_all)]
    pub(crate) fn peek(&mut self, span: &dyn Spanned) -> compile::Result<InstAddress> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        tracing::trace!(?layer);
        Ok(InstAddress::new(self.size))
    }

    /// Perform a linear allocation.
    #[tracing::instrument(skip_all)]
    pub(crate) fn linear(&mut self, span: &dyn Spanned, n: usize) -> compile::Result<Linear> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        let mut addresses = Vec::try_with_capacity(n).with_span(span)?;

        let base = InstAddress::new(self.slots.len());

        for _ in 0..n {
            let addr = self.slots.push().with_span(span)?;
            let address = InstAddress::new(addr);
            addresses.try_push(address)?;
            layer.owned.try_insert(addr).with_span(span)?;
        }

        self.size = self.size.max(self.slots.len());
        Ok(Linear { base, addresses })
    }

    /// Free a bunch of anonymous slots.
    #[tracing::instrument(skip_all, fields(n))]
    pub(crate) fn free(&mut self, span: &dyn Spanned, addr: InstAddress) -> compile::Result<()> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        tracing::trace!(?layer);

        if !layer.owned.remove(&addr.offset()) {
            return Err(compile::Error::msg(
                span,
                format!("Address {addr} is not owned by layer"),
            ));
        }

        if !self.slots.try_remove(addr.offset()) {
            return Err(compile::Error::msg(
                span,
                format!("Address {addr} is not globally allocated"),
            ));
        }

        Ok(())
    }

    /// Free a bunch of linear variables.
    #[tracing::instrument(skip_all, fields(n))]
    pub(crate) fn free_linear(
        &mut self,
        span: &dyn Spanned,
        linear: Linear,
    ) -> compile::Result<()> {
        let Some(layer) = self.layers.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head layer"));
        };

        for addr in linear.iter().rev() {
            if !layer.owned.remove(&addr.offset()) {
                return Err(compile::Error::msg(
                    span,
                    format!("Address {addr} is not owned by layer"),
                ));
            }

            if !self.slots.try_remove(addr.offset()) {
                return Err(compile::Error::msg(
                    span,
                    format!("Address {addr} is not globally allocated"),
                ));
            }
        }

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

        for address in &layer.owned {
            if !self.slots.try_remove(*address) {
                return Err(compile::Error::msg(
                    span,
                    format!(
                        "Address {address} owned by layer {expected} is not globally allocated"
                    ),
                ));
            }
        }

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

    /// Push a scope and return an index.
    pub(crate) fn push(&mut self, layer: Layer<'hir>) -> alloc::Result<ScopeGuard> {
        self.layers.try_push(layer)?;
        Ok(ScopeGuard(self.layers.len()))
    }
}

pub(super) struct Linear {
    base: InstAddress,
    addresses: Vec<InstAddress>,
}

impl Linear {
    #[inline]
    pub(super) fn addr(&self) -> InstAddress {
        self.base
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = InstAddress> + '_ {
        self.addresses.iter().copied()
    }
}

impl<'a> IntoIterator for &'a Linear {
    type Item = InstAddress;
    type IntoIter = iter::Copied<slice::Iter<'a, InstAddress>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.addresses.iter().copied()
    }
}
