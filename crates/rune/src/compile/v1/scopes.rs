use core::cell::{Cell, RefCell};
use core::fmt;

use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap};
use crate::ast::Spanned;
use crate::compile::{self, Assembly, ErrorKind, WithSpan};
use crate::hir;
use crate::query::Query;
use crate::runtime::inst;
use crate::runtime::Output;
use crate::SourceId;

use super::{Address, Any, DisplayNamed, Linear, Slab, Slots};

/// Root scope.
const ROOT: ScopeId = ScopeId { index: 0, id: 0 };

#[derive(Debug)]
pub(super) struct Scope<'hir> {
    /// Parent scope.
    parent: ScopeId,
    /// Scope.
    id: ScopeId,
    /// Named variables.
    names: HashMap<hir::Variable, VarInner<'hir>>,
    /// Slots owned by this scope.
    locals: Dangling,
}

impl Scope<'_> {
    /// Construct a new locals handlers.
    fn new(parent: ScopeId, id: ScopeId) -> Self {
        Self {
            parent,
            id,
            names: HashMap::new(),
            locals: Dangling::default(),
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

/// Dangling address set which keeps track of addresses used and the order in
/// which they are inserted.
#[derive(Default)]
struct Dangling {
    addresses: Vec<Option<inst::Address>>,
    address_to_index: HashMap<inst::Address, usize>,
}

impl Dangling {
    fn clear(&mut self) {
        self.addresses.clear();
        self.address_to_index.clear();
    }

    fn insert(&mut self, addr: inst::Address) -> alloc::Result<()> {
        if self.address_to_index.contains_key(&addr) {
            return Ok(());
        }

        self.address_to_index
            .try_insert(addr, self.addresses.len())?;

        self.addresses.try_push(Some(addr))?;
        Ok(())
    }

    fn remove(&mut self, addr: inst::Address) -> bool {
        if let Some(index) = self.address_to_index.remove(&addr) {
            self.addresses[index] = None;
            true
        } else {
            false
        }
    }

    /// Iterate over addresses.
    #[inline]
    fn addresses(&self) -> impl Iterator<Item = inst::Address> + '_ {
        self.addresses.iter().filter_map(|addr| *addr)
    }
}

impl fmt::Debug for Dangling {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.addresses.fmt(f)
    }
}

pub(crate) struct Scopes<'hir> {
    scopes: RefCell<Slab<Scope<'hir>>>,
    /// Set of addresses that are dangling.
    dangling: RefCell<Dangling>,
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
            dangling: RefCell::new(Dangling::default()),
            source_id,
            size: Cell::new(0),
            slots: RefCell::new(Slots::new()),
            id: Cell::new(1),
            top: Cell::new(ROOT),
        })
    }

    /// Drain dangling addresses into a vector.
    pub(crate) fn drain_dangling_into(&self, out: &mut Vec<inst::Address>) -> alloc::Result<()> {
        let mut dangling = self.dangling.borrow_mut();

        for addr in dangling.addresses.drain(..).flatten() {
            out.try_push(addr)?;
        }

        dangling.clear();
        Ok(())
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
        name: hir::Variable,
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
                span: var.span,
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
        name: hir::Variable,
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
                span: var.span,
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
        name: hir::Variable,
        addr: &Address<'_, 'hir>,
    ) -> compile::Result<()> {
        let mut scopes = self.scopes.borrow_mut();

        let Some(scope) = scopes.get_mut(self.top.get().index) else {
            return Err(compile::Error::msg(span, "Missing top scope"));
        };

        let var = VarInner {
            span,
            name,
            addr: addr.addr(),
            moved_at: Cell::new(None),
        };

        scope.names.try_insert(name, var).with_span(span)?;
        tracing::trace!(?scope, ?name);
        Ok(())
    }

    /// Defer slot allocation.
    #[tracing::instrument(skip_all)]
    pub(super) fn defer(&self, span: &'hir dyn Spanned) -> Any<'_, 'hir> {
        Any::defer(self, self.top.get(), span)
    }

    /// Explicitly allocate a slot.
    #[tracing::instrument(skip_all)]
    pub(super) fn alloc(&self, span: &'hir dyn Spanned) -> compile::Result<Address<'_, 'hir>> {
        let mut scopes = self.scopes.borrow_mut();

        let Some(scope) = scopes.get_mut(self.top.get().index) else {
            return Err(compile::Error::msg(span, "Missing top scope"));
        };

        let addr = inst::Address::new(self.slots.borrow_mut().insert()?);
        self.size.set(self.size.get().max(addr.offset() + 1));
        scope.locals.insert(addr).with_span(span)?;
        self.dangling.borrow_mut().remove(addr);

        tracing::trace!(?scope, ?addr, size = self.size.get());
        Ok(Address::local(span, self, addr))
    }

    /// Declare an anonymous variable.
    #[tracing::instrument(skip(self, span))]
    pub(super) fn alloc_in(
        &self,
        span: &dyn Spanned,
        id: ScopeId,
    ) -> compile::Result<inst::Address> {
        let mut scopes = self.scopes.borrow_mut();

        let Some(scope) = scopes.get_mut(id.index) else {
            return Err(compile::Error::msg(
                span,
                format!("Missing scope {id} to allocate in"),
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

        let addr = inst::Address::new(self.slots.borrow_mut().insert()?);
        scope.locals.insert(addr).with_span(span)?;
        self.size.set(self.size.get().max(addr.offset() + 1));
        self.dangling.borrow_mut().remove(addr);

        tracing::trace!(?scope, ?addr, size = self.size.get());
        Ok(addr)
    }

    /// Perform a linear allocation.
    #[tracing::instrument(ret(level = "trace"), skip(self, span))]
    pub(super) fn linear(
        &self,
        span: &'hir dyn Spanned,
        n: usize,
    ) -> compile::Result<Linear<'_, 'hir>> {
        let mut scopes = self.scopes.borrow_mut();

        let Some(scope) = scopes.get_mut(self.top.get().index) else {
            return Err(compile::Error::msg(span, "Missing top scope"));
        };

        let mut dangling = self.dangling.borrow_mut();

        let linear = match n {
            0 => Linear::empty(),
            1 => {
                let addr = inst::Address::new(self.slots.borrow_mut().insert()?);
                scope.locals.insert(addr).with_span(span)?;
                dangling.remove(addr);
                self.size.set(self.size.get().max(addr.offset() + 1));
                Linear::single(Address::local(span, self, addr))
            }
            n => {
                let mut slots = self.slots.borrow_mut();
                let mut addresses = Vec::try_with_capacity(n).with_span(span)?;

                for _ in 0..n {
                    let addr = inst::Address::new(slots.push().with_span(span)?);
                    scope.locals.insert(addr).with_span(span)?;
                    dangling.remove(addr);
                    addresses.try_push(Address::dangling(span, self, addr))?;
                    self.size.set(self.size.get().max(addr.offset() + 1));
                }

                Linear::new(addresses)
            }
        };

        tracing::trace!(?scope, ?linear, size = self.size.get());
        Ok(linear)
    }

    /// Free an address if it's in the specified scope.
    #[tracing::instrument(skip(self, span))]
    pub(super) fn free_addr(
        &self,
        span: &dyn Spanned,
        addr: inst::Address,
        name: Option<&'static str>,
        dangling: bool,
    ) -> compile::Result<()> {
        let mut scopes = self.scopes.borrow_mut();

        let Some(scope) = scopes.get_mut(self.top.get().index) else {
            return Err(compile::Error::msg(
                span,
                format!(
                    "Freed address {} does not have an implicit scope",
                    DisplayNamed::new(addr, name)
                ),
            ));
        };

        tracing::trace!(?scope);

        if !scope.locals.remove(addr) {
            return Err(compile::Error::msg(
                span,
                format!(
                    "Freed address {} is not defined in scope {}",
                    DisplayNamed::new(addr, name),
                    scope.id
                ),
            ));
        }

        if dangling {
            self.dangling.borrow_mut().insert(addr).with_span(span)?;
        }

        let mut slots = self.slots.borrow_mut();

        if !slots.remove(addr.offset()) {
            return Err(compile::Error::msg(
                span,
                format!(
                    "Freed adddress {} does not have a global slot in scope {}",
                    DisplayNamed::new(addr, name),
                    scope.id
                ),
            ));
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
        let mut dangling = self.dangling.borrow_mut();

        // Free any locally defined variables associated with the scope.
        for addr in scope.locals.addresses() {
            if !slots.remove(addr.offset()) {
                return Err(compile::Error::msg(
                    span,
                    format!(
                        "Address {addr} is not globally allocated in {:?}",
                        self.slots
                    ),
                ));
            }

            dangling.insert(addr).with_span(span)?;
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

/// A locally declared variable, its calculated stack offset and where it was
/// declared in its source file.
pub(super) struct Var<'hir> {
    /// The span where the variable was declared.
    pub(super) span: &'hir dyn Spanned,
    /// The name of the variable.
    name: hir::Variable,
    /// Address where the variable is currently live.
    pub(super) addr: inst::Address,
}

impl fmt::Debug for Var<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Var")
            .field("span", &self.span.span())
            .field("name", &self.name)
            .field("addr", &self.addr)
            .finish()
    }
}

impl fmt::Display for Var<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

impl Var<'_> {
    /// Copy the declared variable.
    pub(super) fn copy(
        &self,
        asm: &mut Assembly,
        span: &dyn Spanned,
        comment: Option<&dyn fmt::Display>,
        out: Output,
    ) -> compile::Result<()> {
        asm.push_with_comment(
            inst::Kind::Copy {
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
            inst::Kind::Move {
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
    /// Token assocaited with the variable.
    span: &'hir dyn Spanned,
    /// The name of the variable.
    name: hir::Variable,
    /// Offset from the current stack frame.
    addr: inst::Address,
    /// Variable has been taken at the given position.
    moved_at: Cell<Option<&'hir dyn Spanned>>,
}

impl fmt::Debug for VarInner<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Var")
            .field("span", &self.span.span())
            .field("name", &self.name)
            .field("addr", &self.addr)
            .field("moved_at", &self.moved_at.get().map(|s| s.span()))
            .finish()
    }
}
