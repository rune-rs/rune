use core::cell::{Cell, RefCell};
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
pub(super) struct Scope<'hir> {
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

/// A scope handle which does not implement Copy to make it harder to misuse.
#[must_use = "Scope handles must be handed back to Scopes to be freed"]
pub(super) struct ScopeHandle {
    pub(super) id: ScopeId,
}

/// A scope that has been popped but not freed.
#[must_use = "Scope handles must be handed back to Scopes to be freed"]
pub(super) struct DanglingScope {
    pub(super) id: ScopeId,
}

/// A guard returned from [push][Scopes::push].
///
/// This should be provided to a subsequent [pop][Scopes::pop] to allow it to be
/// sanity checked.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub(super) struct ScopeId {
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
    scopes: RefCell<Slab<Scope<'hir>>>,
    source_id: SourceId,
    size: Cell<usize>,
    slots: RefCell<Slots>,
    id: Cell<usize>,
    top: Cell<ScopeId>,
}

impl<'hir> Scopes<'hir> {
    /// Construct a new collection of scopes.
    pub(crate) fn new(source_id: SourceId) -> alloc::Result<Self> {
        let mut scopes = Slab::new();
        scopes.insert(Scope::new(ROOT, ROOT))?;

        Ok(Self {
            scopes: RefCell::new(scopes),
            source_id,
            size: Cell::new(0),
            slots: RefCell::new(Slots::new()),
            id: Cell::new(1),
            top: Cell::new(ROOT),
        })
    }

    /// Get the maximum total number of variables used in a function.
    /// Effectively the required stack size.
    #[inline]
    pub(crate) fn size(&self) -> usize {
        self.size.get()
    }

    /// Get the last scope guard.
    #[inline]
    pub(super) fn top_id(&self) -> ScopeId {
        self.top.get()
    }

    /// Get the local with the given name.
    #[tracing::instrument(skip(self, q, span))]
    pub(super) fn get(
        &self,
        q: &mut Query<'_, '_>,
        span: &dyn Spanned,
        name: hir::Name<'hir>,
    ) -> compile::Result<Var<'hir>> {
        let scopes = self.scopes.borrow();
        let mut current = Some(self.top.get());

        while let Some(id) = current.take() {
            let Some(scope) = scopes.get(id.index) else {
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
    pub(super) fn take(
        &self,
        q: &mut Query<'_, '_>,
        span: &'hir dyn Spanned,
        name: hir::Name<'hir>,
    ) -> compile::Result<Var<'hir>> {
        let scopes = self.scopes.borrow();
        let mut current = Some(self.top.get());

        while let Some(id) = current.take() {
            let Some(scope) = scopes.get(id.index) else {
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
    pub(super) fn define(
        &self,
        span: &'hir dyn Spanned,
        name: hir::Name<'hir>,
        addr: InstAddress,
    ) -> compile::Result<()> {
        let mut scopes = self.scopes.borrow_mut();

        let Some(scope) = scopes.get_mut(self.top.get().index) else {
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
    pub(super) fn alloc(&self, span: &'hir dyn Spanned) -> compile::Result<NeedsAddress<'hir>> {
        let mut scopes = self.scopes.borrow_mut();

        let Some(scope) = scopes.get_mut(self.top.get().index) else {
            return Err(compile::Error::msg(span, "Missing top scope"));
        };

        let offset = self.slots.borrow_mut().insert()?;

        scope.locals.try_insert(offset).with_span(span)?;
        self.size.set(self.size.get().max(offset + 1));
        let addr = InstAddress::new(offset);
        tracing::trace!(?scope, ?addr, size = self.size.get());
        Ok(NeedsAddress::with_local(span, addr))
    }

    /// Declare an anonymous variable.
    #[tracing::instrument(skip(self, span))]
    pub(super) fn alloc_in(&self, span: &dyn Spanned, id: ScopeId) -> compile::Result<InstAddress> {
        let mut scopes = self.scopes.borrow_mut();

        let Some(s) = scopes.get_mut(id.index) else {
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

        let offset = self.slots.borrow_mut().insert()?;

        s.locals.try_insert(offset).with_span(span)?;
        self.size.set(self.size.get().max(offset + 1));
        let addr = InstAddress::new(offset);
        tracing::trace!(?s, ?addr, size = self.size.get());
        Ok(addr)
    }

    /// Perform a linear allocation.
    #[tracing::instrument(ret, skip(self, span))]
    pub(super) fn linear<'a>(
        &self,
        span: &'a dyn Spanned,
        n: usize,
    ) -> compile::Result<Linear<'a>> {
        let mut scopes = self.scopes.borrow_mut();

        let Some(scope) = scopes.get_mut(self.top.get().index) else {
            return Err(compile::Error::msg(span, "Missing top scope"));
        };

        if n == 0 {
            return Ok(Linear {
                base: InstAddress::INVALID,
                addresses: Vec::new(),
            });
        }

        let mut slots = self.slots.borrow_mut();
        let mut addresses = Vec::try_with_capacity(n).with_span(span)?;

        for _ in 0..n {
            let offset = slots.push().with_span(span)?;
            let address = InstAddress::new(offset);
            addresses.try_push(NeedsAddress::with_reserved(span, address))?;
            scope.locals.try_insert(offset).with_span(span)?;
            self.size.set(self.size.get().max(offset + 1));
        }

        let base = addresses
            .first()
            .map(|a| a.addr())
            .unwrap_or(InstAddress::INVALID);

        let linear = Linear { base, addresses };
        tracing::trace!(?scope, ?linear, size = self.size.get());
        Ok(linear)
    }

    /// Free an address if it's in the specified scope.
    #[tracing::instrument(skip(self))]
    pub(super) fn free(&self, addr: NeedsAddress<'hir>) -> compile::Result<()> {
        match &addr.kind {
            NeedsAddressKind::Local | NeedsAddressKind::Dangling => {
                self.free_addr(addr.span, addr.addr(), addr.name)?;
            }
            NeedsAddressKind::Scope(scope) => {
                if self.top_id() == *scope {
                    self.free_addr(addr.span, addr.addr(), addr.name)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Free an address if it's in the specified scope.
    #[tracing::instrument(skip(self, span))]
    pub(super) fn free_addr(
        &self,
        span: &dyn Spanned,
        addr: InstAddress,
        name: Option<&'static str>,
    ) -> compile::Result<()> {
        let mut scopes = self.scopes.borrow_mut();

        let Some(scope) = scopes.get_mut(self.top.get().index) else {
            return Err(compile::Error::msg(
                span,
                format!(
                    "Freed address {} does not have an implicit scope",
                    DisplayAddr(addr, name)
                ),
            ));
        };

        tracing::trace!(?scope);

        if !scope.locals.remove(&addr.offset()) {
            return Err(compile::Error::msg(
                span,
                format!(
                    "Freed address {} is not defined in scope {}",
                    DisplayAddr(addr, name),
                    scope.id
                ),
            ));
        }

        let mut slots = self.slots.borrow_mut();

        if !slots.remove(addr.offset()) {
            return Err(compile::Error::msg(
                span,
                format!(
                    "Freed adddress {} does not have a global slot in scope {}",
                    DisplayAddr(addr, name),
                    scope.id
                ),
            ));
        }

        Ok(())
    }

    /// Free a bunch of linear variables.
    #[tracing::instrument(skip(self, linear), fields(linear.base, len = linear.len()))]
    pub(super) fn free_linear(&self, linear: Linear<'hir>) -> compile::Result<()> {
        for addr in linear.addresses.into_iter().rev() {
            self.free(addr)?;
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, span, handle), fields(id = ?handle.id))]
    pub(super) fn pop(&self, span: &dyn Spanned, handle: ScopeHandle) -> compile::Result<()> {
        let ScopeHandle { id } = handle;

        let Some(mut scope) = self.scopes.borrow_mut().try_remove(id.index) else {
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

        let mut slots = self.slots.borrow_mut();

        // Free any locally defined variables associated with the scope.
        for addr in &scope.locals {
            if !slots.remove(*addr) {
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
        self.top.set(scope.parent);
        Ok(())
    }

    /// Pop the last of the scope.
    #[tracing::instrument(skip(self, span))]
    pub(super) fn pop_last(&self, span: &dyn Spanned) -> compile::Result<()> {
        self.pop(span, ScopeHandle { id: ROOT })?;
        Ok(())
    }

    /// Construct a new child scope and return its guard.
    #[tracing::instrument(skip_all)]
    pub(super) fn child(&self, span: &dyn Spanned) -> compile::Result<ScopeHandle> {
        let mut scopes = self.scopes.borrow_mut();

        let id = ScopeId {
            index: scopes.vacant_key(),
            id: self.id.replace(self.id.get().wrapping_add(1)),
        };

        let scope = Scope::new(self.top.replace(id), id);
        tracing::trace!(?scope);
        scopes.insert(scope).with_span(span)?;
        Ok(ScopeHandle { id })
    }

    #[tracing::instrument(skip(self, span, handle), fields(id = ?handle.id))]
    pub(super) fn dangle(
        &self,
        span: &dyn Spanned,
        handle: ScopeHandle,
    ) -> compile::Result<DanglingScope> {
        let ScopeHandle { id } = handle;

        let scopes = self.scopes.borrow();

        let Some(scope) = scopes.get(id.index) else {
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

        self.top.set(scope.parent);
        tracing::trace!(?scope);
        Ok(DanglingScope { id })
    }

    /// Push a dangling scope back onto the stack.
    #[tracing::instrument(skip_all)]
    pub(super) fn restore(&self, handle: DanglingScope) -> ScopeHandle {
        let DanglingScope { id } = handle;
        tracing::trace!(?id);
        self.top.set(id);
        ScopeHandle { id }
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
    pub(super) fn iter(&self) -> slice::Iter<'_, NeedsAddress<'hir>> {
        self.addresses.iter()
    }

    #[inline]
    pub(super) fn iter_mut(&mut self) -> slice::IterMut<'_, NeedsAddress<'hir>> {
        self.addresses.iter_mut()
    }

    #[inline]
    pub(super) fn free(self, scopes: &Scopes<'hir>) -> compile::Result<()> {
        scopes.free_linear(self)
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
    pub(super) addr: InstAddress,
}

impl<'hir> fmt::Display for Var<'hir> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

impl<'hir> Var<'hir> {
    /// Copy the declared variable.
    pub(super) fn copy(
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
    pub(super) fn move_(
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

struct DisplayAddr(InstAddress, Option<&'static str>);

impl fmt::Display for DisplayAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.1 {
            Some(name) => write!(f, "{} ({})", self.0, name),
            None => self.0.fmt(f),
        }
    }
}
