use core::cell::Cell;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::slice;

use crate::alloc::prelude::*;
use crate::alloc::{self, BTreeSet, HashMap};
use crate::ast::Spanned;
use crate::compile::v1::Ctxt;
use crate::compile::{self, Assembly, ErrorKind, WithSpan};
use crate::hir;
use crate::query::Query;
use crate::runtime::{Inst, InstAddress, Output};
use crate::SourceId;

use super::{Needs, NeedsAddressKind, NeedsKind, Slab};

#[derive(Debug)]
pub(crate) struct Scope<'hir> {
    id: ScopeId,
    /// Named variables.
    names: HashMap<hir::Name<'hir>, VarInner<'hir>>,
    /// Slots owned by this scope.
    locals: BTreeSet<usize>,
}

impl<'hir> Scope<'hir> {
    /// Construct a new locals handlers.
    fn new(id: ScopeId) -> Self {
        Self {
            id,
            names: HashMap::new(),
            locals: BTreeSet::new(),
        }
    }
}

/// A guard returned from [push][Scopes::push].
///
/// This should be provided to a subsequent [pop][Scopes::pop] to allow it to be
/// sanity checked.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub(crate) struct ScopeId(usize, usize);

impl fmt::Display for ScopeId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.0, self.1)
    }
}

impl fmt::Debug for ScopeId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub(crate) struct Scopes<'hir> {
    scopes: Vec<Scope<'hir>>,
    source_id: SourceId,
    size: usize,
    slots: Slab,
    id: usize,
}

impl<'hir> Scopes<'hir> {
    /// Construct a new collection of scopes.
    pub(crate) fn new(source_id: SourceId) -> alloc::Result<Self> {
        Ok(Self {
            scopes: try_vec![Scope::new(ScopeId(0, 0))],
            source_id,
            size: 0,
            slots: Slab::new(),
            id: 1,
        })
    }

    /// Get the maximum total number of variables used in a function.
    /// Effectively the required stack size.
    #[inline]
    pub(crate) fn size(&self) -> usize {
        self.size
    }

    /// Get the last scope guard.
    #[inline]
    pub(crate) fn top_id(&self) -> Option<ScopeId> {
        Some(self.scopes.last()?.id)
    }

    /// Get the local with the given name.
    #[tracing::instrument(skip(self, q, span))]
    pub(crate) fn get(
        &self,
        q: &mut Query<'_, '_>,
        span: &dyn Spanned,
        name: hir::Name<'hir>,
    ) -> compile::Result<Var<'hir>> {
        for scope in self.scopes.iter().rev() {
            let Some(var) = scope.names.get(&name) else {
                continue;
            };

            if let Some(_moved_at) = var.moved_at.get() {
                return Err(compile::Error::new(
                    span,
                    ErrorKind::VariableMoved {
                        #[cfg(feature = "emit")]
                        moved_at: _moved_at.span(),
                    },
                ));
            }

            q.visitor
                .visit_variable_use(self.source_id, var.span, span)
                .with_span(span)?;

            let var = Var {
                name: var.name,
                addr: var.addr,
            };

            tracing::trace!(?scope, ?var);
            return Ok(var);
        }

        Err(compile::Error::msg(
            span,
            try_format!("Missing variable `{name}`"),
        ))
    }

    /// Take the local with the given name.
    #[tracing::instrument(skip(self, q, span))]
    pub(crate) fn take(
        &self,
        q: &mut Query<'_, '_>,
        span: &'hir dyn Spanned,
        name: hir::Name<'hir>,
    ) -> compile::Result<Var<'hir>> {
        for scope in self.scopes.iter().rev() {
            let Some(var) = scope.names.get(&name) else {
                continue;
            };

            if let Some(_moved_at) = var.moved_at.get() {
                return Err(compile::Error::new(
                    span,
                    ErrorKind::VariableMoved {
                        #[cfg(feature = "emit")]
                        moved_at: _moved_at.span(),
                    },
                ));
            }

            q.visitor
                .visit_variable_use(self.source_id, var.span, span)
                .with_span(span)?;

            var.moved_at.set(Some(span));

            let var = Var {
                name: var.name,
                addr: var.addr,
            };

            tracing::trace!(?scope, ?var);
            return Ok(var);
        }

        Err(compile::Error::msg(
            span,
            try_format!("Missing variable `{name}` to take"),
        ))
    }

    /// Construct a new variable.
    #[tracing::instrument(skip(self, span))]
    pub(crate) fn define(
        &mut self,
        span: &'hir dyn Spanned,
        name: hir::Name<'hir>,
        addr: InstAddress,
    ) -> compile::Result<()> {
        let Some(scope) = self.scopes.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head scope"));
        };

        let var = VarInner {
            name,
            addr,
            span,
            moved_at: Cell::new(None),
        };

        scope.names.try_insert(name, var).with_span(span)?;
        tracing::trace!(?scope, ?name);
        Ok(())
    }

    /// Declare an anonymous variable.
    #[tracing::instrument(skip_all)]
    pub(crate) fn alloc(&mut self, span: &'hir dyn Spanned) -> compile::Result<Needs<'hir>> {
        let Some(scope) = self.scopes.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head scope"));
        };

        let offset = self.slots.insert()?;

        scope.locals.try_insert(offset).with_span(span)?;
        self.size = self.size.max(self.slots.len());
        let addr = InstAddress::new(offset);
        tracing::trace!(?scope, ?addr, self.size);
        Ok(Needs::with_addr(span, addr))
    }

    /// Declare an anonymous variable.
    #[tracing::instrument(skip(self, span))]
    pub(crate) fn alloc_in(
        &mut self,
        span: &dyn Spanned,
        scope: ScopeId,
    ) -> compile::Result<InstAddress> {
        let ScopeId(_, index) = scope;

        let Some(s) = self.scopes.get_mut(index) else {
            return Err(compile::Error::msg(
                span,
                format!("Missing scope {scope} to allocate in"),
            ));
        };

        if s.id != scope {
            return Err(compile::Error::msg(
                span,
                try_format!("Scope id mismatch, {} (actual) != {scope} (expected)", s.id),
            ));
        }

        let offset = self.slots.insert()?;

        s.locals.try_insert(offset).with_span(span)?;
        self.size = self.size.max(self.slots.len());
        let addr = InstAddress::new(offset);
        tracing::trace!(?s, ?addr, self.size);
        Ok(addr)
    }

    /// Perform a linear allocation.
    #[tracing::instrument(skip(self, span))]
    pub(crate) fn linear<'a>(
        &mut self,
        span: &'a dyn Spanned,
        n: usize,
    ) -> compile::Result<Linear<'a>> {
        let Some(scope) = self.scopes.last_mut() else {
            return Err(compile::Error::msg(span, "Missing head scope"));
        };

        let base = InstAddress::new(self.slots.len());

        if n == 0 {
            return Ok(Linear {
                base: InstAddress::new(self.slots.len()),
                addresses: Vec::new(),
            });
        }

        let mut addresses = Vec::try_with_capacity(n).with_span(span)?;

        for _ in 0..n {
            let addr = self.slots.push().with_span(span)?;
            let address = InstAddress::new(addr);
            addresses.try_push(Needs::with_addr(span, address))?;
            scope.locals.try_insert(addr).with_span(span)?;
        }

        self.size = self.size.max(self.slots.len());
        tracing::trace!(?scope, ?base, ?addresses, self.size);
        Ok(Linear { base, addresses })
    }

    /// Free an address if it's in the specified scope.
    #[tracing::instrument(skip(self))]
    pub(crate) fn free(&mut self, needs: Needs<'_>) -> compile::Result<()> {
        let NeedsKind::Address(addr) = &needs.kind else {
            return Ok(());
        };

        match &addr.kind {
            NeedsAddressKind::Local => {
                self.free_addr(needs.span, addr.addr)?;
            }
            NeedsAddressKind::Scope(scope) => {
                if self.top_id() == Some(*scope) {
                    self.free_addr(needs.span, addr.addr)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Free an address if it's in the specified scope.
    #[tracing::instrument(skip(self, span))]
    pub(crate) fn free_addr(
        &mut self,
        span: &dyn Spanned,
        addr: InstAddress,
    ) -> compile::Result<()> {
        let Some(scope) = self.scopes.last_mut() else {
            return Err(compile::Error::msg(
                span,
                format!("Missing scope when freeing address {addr:?}"),
            ));
        };

        if !scope.locals.remove(&addr.offset()) {
            return Err(compile::Error::msg(
                span,
                format!("Address {addr} is not defined in scope {}", scope.id),
            ));
        }

        if !self.slots.remove(addr.offset()) {
            return Err(compile::Error::msg(
                span,
                format!(
                    "Address {addr} is not globally allocated in scope {}",
                    scope.id
                ),
            ));
        }

        tracing::trace!(?scope);
        Ok(())
    }

    /// Free a bunch of linear variables.
    #[tracing::instrument(skip(self))]
    pub(crate) fn free_linear(&mut self, linear: Linear<'_>) -> compile::Result<()> {
        for needs in linear.addresses.into_iter().rev() {
            self.free(needs)?;
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, span))]
    pub(crate) fn pop(
        &mut self,
        span: &dyn Spanned,
        scope: ScopeId,
    ) -> compile::Result<Scope<'hir>> {
        let Some(mut s) = self.scopes.pop() else {
            return Err(compile::Error::msg(
                span,
                format!("Missing scope while expected {scope}"),
            ));
        };

        if s.id != scope {
            return Err(compile::Error::msg(
                span,
                try_format!("Scope id mismatch, {} (actual) != {scope} (expected)", s.id),
            ));
        }

        // Free any locally defined variables associated with the scope.
        for address in &s.locals {
            if !self.slots.remove(*address) {
                return Err(compile::Error::msg(
                    span,
                    format!("Address {address} is not globally allocated"),
                ));
            }
        }

        tracing::trace!(?s);
        s.locals.clear();
        Ok(s)
    }

    /// Pop the last of the scope.
    pub(crate) fn pop_last(&mut self, span: &dyn Spanned) -> compile::Result<()> {
        self.pop(span, ScopeId(0, 0))?;
        Ok(())
    }

    /// Construct a new child scope and return its guard.
    #[tracing::instrument(skip_all)]
    pub(crate) fn child(&mut self, span: &dyn Spanned) -> compile::Result<ScopeId> {
        let id = ScopeId(self.id, self.scopes.len());
        self.id += 1;
        let scope = Scope::new(id);
        tracing::trace!(?scope);
        self.scopes.try_push(scope).with_span(span)?;
        Ok(id)
    }

    /// Push a scope again.
    #[tracing::instrument(skip_all)]
    pub(crate) fn push(&mut self, scope: Scope<'hir>) -> compile::Result<ScopeId> {
        tracing::trace!(?scope);
        let id = scope.id;
        self.scopes.try_push(scope)?;
        Ok(id)
    }
}

#[derive(Debug)]
#[must_use = "This should be freed with a call to Scopes::free_linear"]
pub(super) struct Linear<'a> {
    base: InstAddress,
    addresses: Vec<Needs<'a>>,
}

impl<'a> Linear<'a> {
    /// Construct an empty linear allocation.
    ///
    /// In practice, the exact address should not matter.
    pub(super) fn empty(base: InstAddress) -> Self {
        Self {
            base,
            addresses: Vec::new(),
        }
    }

    #[inline]
    pub(super) fn addr(&self) -> InstAddress {
        self.base
    }

    #[inline]
    pub(crate) fn iter(&self) -> slice::Iter<'_, Needs<'a>> {
        self.addresses.iter()
    }

    #[inline]
    pub(crate) fn iter_mut(&mut self) -> slice::IterMut<'_, Needs<'a>> {
        self.addresses.iter_mut()
    }
}

impl<'a, 'b> IntoIterator for &'a Linear<'b> {
    type Item = &'a Needs<'b>;
    type IntoIter = slice::Iter<'a, Needs<'b>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, 'b> IntoIterator for &'a mut Linear<'b> {
    type Item = &'a mut Needs<'b>;
    type IntoIter = slice::IterMut<'a, Needs<'b>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'hir> Deref for Linear<'hir> {
    type Target = [Needs<'hir>];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.addresses
    }
}

impl<'hir> DerefMut for Linear<'hir> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.addresses
    }
}

/// A locally declared variable, its calculated stack offset and where it was
/// declared in its source file.
#[derive(Debug)]
pub struct Var<'hir> {
    /// The name of the variable.
    name: hir::Name<'hir>,
    /// Offset from the current stack frame.
    pub(crate) addr: InstAddress,
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
    pub(crate) fn move_(
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

/// A locally declared variable, its calculated stack offset and where it was
/// declared in its source file.
struct VarInner<'hir> {
    /// The name of the variable.
    name: hir::Name<'hir>,
    /// Offset from the current stack frame.
    addr: InstAddress,
    /// Token assocaited with the variable.
    span: &'hir dyn Spanned,
    /// Variable has been taken at the given position.
    moved_at: Cell<Option<&'hir dyn Spanned>>,
}

impl fmt::Debug for VarInner<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Var")
            .field("name", &self.name)
            .field("addr", &self.addr)
            .field("span", &self.span.span())
            .field("moved_at", &self.moved_at.get().map(|s| s.span()))
            .finish()
    }
}
