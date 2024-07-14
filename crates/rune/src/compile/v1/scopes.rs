use core::cell::Cell;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::slice;

use crate::alloc::prelude::*;
use crate::alloc::{self, BTreeSet, HashMap};
use crate::ast::Spanned;
use crate::compile::{self, Assembly, ErrorKind, WithSpan};
use crate::hir;
use crate::query::Query;
use crate::runtime::{Inst, InstAddress, Output};
use crate::SourceId;

use super::{NeedsAddress, NeedsAddressKind, Slab, Slots};

/// Root scope.
const ROOT: ScopeId = ScopeId { index: 0, id: 0 };

#[derive(Debug)]
pub(crate) struct Scope<'hir> {
    /// Parent scope.
    parent: ScopeId,
    /// Scope.
    id: ScopeId,
    /// Named variables.
    names: HashMap<hir::Name<'hir>, VarInner<'hir>>,
    /// Slots owned by this scope.
    locals: BTreeSet<usize>,
}

impl<'hir> Scope<'hir> {
    /// Construct a new locals handlers.
    fn new(parent: ScopeId, id: ScopeId) -> Self {
        Self {
            parent,
            id,
            names: HashMap::new(),
            locals: BTreeSet::new(),
        }
    }

    /// Get the parent scope.
    fn parent(&self) -> Option<ScopeId> {
        (self.parent != self.id).then_some(self.parent)
    }
}

/// A guard returned from [push][Scopes::push].
///
/// This should be provided to a subsequent [pop][Scopes::pop] to allow it to be
/// sanity checked.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub(crate) struct ScopeId {
    index: usize,
    id: usize,
}

impl fmt::Display for ScopeId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.index, self.id)
    }
}

impl fmt::Debug for ScopeId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub(crate) struct Scopes<'hir> {
    scopes: Slab<Scope<'hir>>,
    source_id: SourceId,
    size: usize,
    slots: Slots,
    id: usize,
    top: ScopeId,
}

impl<'hir> Scopes<'hir> {
    /// Construct a new collection of scopes.
    pub(crate) fn new(source_id: SourceId) -> alloc::Result<Self> {
        let mut scopes = Slab::new();
        scopes.insert(Scope::new(ROOT, ROOT))?;

        Ok(Self {
            scopes,
            source_id,
            size: 0,
            slots: Slots::new(),
            id: 1,
            top: ROOT,
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
    pub(crate) fn top_id(&self) -> ScopeId {
        self.top
    }

    /// Get the local with the given name.
    #[tracing::instrument(skip(self, q, span))]
    pub(crate) fn get(
        &self,
        q: &mut Query<'_, '_>,
        span: &dyn Spanned,
        name: hir::Name<'hir>,
    ) -> compile::Result<Var<'hir>> {
        let mut current = Some(self.top);

        while let Some(id) = current.take() {
            let Some(scope) = self.scopes.get(id.index) else {
                return Err(compile::Error::msg(span, format!("Missing scope {id}")));
            };

            current = scope.parent();

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
        let mut current = Some(self.top);

        while let Some(id) = current.take() {
            let Some(scope) = self.scopes.get(id.index) else {
                return Err(compile::Error::msg(span, format!("Missing scope {id}")));
            };

            current = scope.parent();

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
        let Some(scope) = self.scopes.get_mut(self.top.index) else {
            return Err(compile::Error::msg(span, "Missing top scope"));
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
    pub(crate) fn alloc(&mut self, span: &'hir dyn Spanned) -> compile::Result<NeedsAddress<'hir>> {
        let Some(scope) = self.scopes.get_mut(self.top.index) else {
            return Err(compile::Error::msg(span, "Missing top scope"));
        };

        let offset = self.slots.insert()?;

        scope.locals.try_insert(offset).with_span(span)?;
        self.size = self.size.max(offset + 1);
        let addr = InstAddress::new(offset);
        tracing::trace!(?scope, ?addr, self.size);
        Ok(NeedsAddress::with_local(span, addr))
    }

    /// Declare an anonymous variable.
    #[tracing::instrument(skip(self, span))]
    pub(crate) fn alloc_in(
        &mut self,
        span: &dyn Spanned,
        id: ScopeId,
    ) -> compile::Result<InstAddress> {
        let Some(s) = self.scopes.get_mut(id.index) else {
            return Err(compile::Error::msg(
                span,
                format!("Missing scope {id} to allocate in"),
            ));
        };

        if s.id != id {
            return Err(compile::Error::msg(
                span,
                try_format!("Scope id mismatch, {} (actual) != {id} (expected)", s.id),
            ));
        }

        let offset = self.slots.insert()?;

        s.locals.try_insert(offset).with_span(span)?;
        self.size = self.size.max(offset + 1);
        let addr = InstAddress::new(offset);
        tracing::trace!(?s, ?addr, self.size);
        Ok(addr)
    }

    /// Perform a linear allocation.
    #[tracing::instrument(ret, skip(self, span))]
    pub(crate) fn linear<'a>(
        &mut self,
        span: &'a dyn Spanned,
        n: usize,
    ) -> compile::Result<Linear<'a>> {
        let Some(scope) = self.scopes.get_mut(self.top.index) else {
            return Err(compile::Error::msg(span, "Missing top scope"));
        };

        if n == 0 {
            return Ok(Linear {
                base: InstAddress::INVALID,
                addresses: Vec::new(),
            });
        }

        let mut addresses = Vec::try_with_capacity(n).with_span(span)?;

        for _ in 0..n {
            let offset = self.slots.push().with_span(span)?;
            let address = InstAddress::new(offset);
            addresses.try_push(NeedsAddress::with_reserved(span, address))?;
            scope.locals.try_insert(offset).with_span(span)?;
            self.size = self.size.max(offset + 1);
        }

        let base = addresses
            .first()
            .map(|a| a.addr())
            .unwrap_or(InstAddress::INVALID);

        let linear = Linear { base, addresses };
        tracing::trace!(?scope, ?linear, self.size);
        Ok(linear)
    }

    /// Free an address if it's in the specified scope.
    #[tracing::instrument(skip(self))]
    pub(crate) fn free(&mut self, addr: NeedsAddress<'hir>) -> compile::Result<()> {
        match &addr.kind {
            NeedsAddressKind::Local | NeedsAddressKind::Dangling => {
                self.free_addr(addr.span, addr.addr())?;
            }
            NeedsAddressKind::Scope(scope) => {
                if self.top_id() == *scope {
                    self.free_addr(addr.span, addr.addr())?;
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
        let Some(scope) = self.scopes.get_mut(self.top.index) else {
            return Err(compile::Error::msg(
                span,
                format!("Freed address {addr} does not have an implicit scope"),
            ));
        };

        tracing::trace!(?scope);

        if !scope.locals.remove(&addr.offset()) {
            return Err(compile::Error::msg(
                span,
                format!("Freed address {addr} is not defined in scope {}", scope.id),
            ));
        }

        if !self.slots.remove(addr.offset()) {
            return Err(compile::Error::msg(
                span,
                format!(
                    "Freed adddress {addr} does not have a global slot in scope {}",
                    scope.id
                ),
            ));
        }

        Ok(())
    }

    /// Free a bunch of linear variables.
    #[tracing::instrument(skip(self, linear), fields(linear.base, len = linear.len()))]
    pub(crate) fn free_linear(&mut self, linear: Linear<'hir>) -> compile::Result<()> {
        for addr in linear.addresses.into_iter().rev() {
            self.free(addr)?;
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, span))]
    pub(crate) fn pop(&mut self, span: &dyn Spanned, id: ScopeId) -> compile::Result<()> {
        let Some(mut scope) = self.scopes.try_remove(id.index) else {
            return Err(compile::Error::msg(
                span,
                format!("Missing scope while expected {id}"),
            ));
        };

        if scope.id != id {
            return Err(compile::Error::msg(
                span,
                try_format!(
                    "Scope id mismatch, {} (actual) != {id} (expected)",
                    scope.id
                ),
            ));
        }

        tracing::trace!(?scope, "freeing locals");

        // Free any locally defined variables associated with the scope.
        for addr in &scope.locals {
            if !self.slots.remove(*addr) {
                return Err(compile::Error::msg(
                    span,
                    format!(
                        "Address {addr} is not globally allocated in {:?}",
                        self.slots
                    ),
                ));
            }
        }

        scope.locals.clear();

        self.top = scope.parent;
        Ok(())
    }

    /// Pop the last of the scope.
    #[tracing::instrument(skip(self, span))]
    pub(crate) fn pop_last(&mut self, span: &dyn Spanned) -> compile::Result<()> {
        self.pop(span, ROOT)?;
        Ok(())
    }

    /// Construct a new child scope and return its guard.
    #[tracing::instrument(skip_all)]
    pub(crate) fn child(&mut self, span: &dyn Spanned) -> compile::Result<ScopeId> {
        let id = ScopeId {
            index: self.scopes.vacant_key(),
            id: self.id,
        };
        self.id += 1;
        let scope = Scope::new(self.top, id);
        tracing::trace!(?scope);
        self.scopes.insert(scope).with_span(span)?;
        self.top = id;
        Ok(id)
    }

    #[tracing::instrument(skip(self, span))]
    pub(crate) fn pop_id(&mut self, span: &dyn Spanned, id: ScopeId) -> compile::Result<()> {
        let Some(scope) = self.scopes.get(id.index) else {
            return Err(compile::Error::msg(
                span,
                format!("Missing scope while expected {id}"),
            ));
        };

        if scope.id != id {
            return Err(compile::Error::msg(
                span,
                try_format!(
                    "Scope id mismatch, {} (actual) != {id} (expected)",
                    scope.id
                ),
            ));
        }

        self.top = scope.parent;
        tracing::trace!(?scope);
        Ok(())
    }

    /// Push a scope again.
    #[tracing::instrument(skip_all)]
    pub(crate) fn push(&mut self, id: ScopeId) {
        tracing::trace!(?id);
        self.top = id;
    }
}

#[derive(Debug)]
#[must_use = "This should be freed with a call to Scopes::free_linear"]
pub(super) struct Linear<'hir> {
    base: InstAddress,
    addresses: Vec<NeedsAddress<'hir>>,
}

impl<'hir> Linear<'hir> {
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
    pub(crate) fn iter(&self) -> slice::Iter<'_, NeedsAddress<'hir>> {
        self.addresses.iter()
    }

    #[inline]
    pub(crate) fn iter_mut(&mut self) -> slice::IterMut<'_, NeedsAddress<'hir>> {
        self.addresses.iter_mut()
    }
}

impl<'a, 'hir> IntoIterator for &'a Linear<'hir> {
    type Item = &'a NeedsAddress<'hir>;
    type IntoIter = slice::Iter<'a, NeedsAddress<'hir>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, 'hir> IntoIterator for &'a mut Linear<'hir> {
    type Item = &'a mut NeedsAddress<'hir>;
    type IntoIter = slice::IterMut<'a, NeedsAddress<'hir>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'hir> Deref for Linear<'hir> {
    type Target = [NeedsAddress<'hir>];

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
        asm: &mut Assembly,
        span: &dyn Spanned,
        comment: Option<&dyn fmt::Display>,
        out: Output,
    ) -> compile::Result<()> {
        asm.push_with_comment(
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
