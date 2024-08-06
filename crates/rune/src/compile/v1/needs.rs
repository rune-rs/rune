use core::fmt;
use core::mem::replace;

use crate::ast::Spanned;
use crate::compile;
use crate::runtime::{Inst, InstAddress, Output};
use crate::shared::{rune_diagnose, Backtrace};

use super::{Ctxt, DisplayNamed, ScopeId, Scopes};

/// Trait used to abstract interactions over different needs.
pub(super) trait Needs<'a, 'hir> {
    /// Access the span for the needs.
    fn span(&self) -> &'hir dyn Spanned;

    /// Get output of the needs or error.
    fn output(&self) -> compile::Result<Output>;

    /// Get the need as an output.
    ///
    /// Returns `None` if `Needs::None` is set.
    fn try_alloc_output(&mut self) -> compile::Result<Option<Output>>;

    fn assign_addr(
        &mut self,
        cx: &mut Ctxt<'_, 'hir, '_>,
        from: InstAddress,
    ) -> compile::Result<()>;

    /// Allocate an output falling back to discarding if one is not available.
    fn alloc_output(&mut self) -> compile::Result<Output>;

    /// Try to allocate an address from this needs.
    fn try_alloc_addr(&mut self) -> compile::Result<Option<&mut Address<'a, 'hir>>>;

    /// Get the need as an address without trying to allocate it if it's
    /// missing.
    fn try_as_addr(&self) -> compile::Result<Option<&Address<'a, 'hir>>>;

    /// Get the populated address of the need.
    fn addr(&self) -> compile::Result<&Address<'a, 'hir>> {
        let Some(addr) = self.try_as_addr()? else {
            return Err(compile::Error::msg(
                self.span(),
                "Expected need to be populated",
            ));
        };

        Ok(addr)
    }
}

impl<'a, 'hir> Needs<'a, 'hir> for Any<'a, 'hir> {
    #[inline]
    fn span(&self) -> &'hir dyn Spanned {
        self.span
    }

    #[inline]
    fn output(&self) -> compile::Result<Output> {
        Any::output(self)
    }

    #[inline]
    fn assign_addr(
        &mut self,
        cx: &mut Ctxt<'_, 'hir, '_>,
        from: InstAddress,
    ) -> compile::Result<()> {
        Any::assign_addr(self, cx, from)
    }

    #[inline]
    fn alloc_output(&mut self) -> compile::Result<Output> {
        Any::alloc_output(self)
    }

    #[inline]
    fn try_alloc_output(&mut self) -> compile::Result<Option<Output>> {
        Any::try_alloc_output(self)
    }

    #[inline]
    fn try_alloc_addr(&mut self) -> compile::Result<Option<&mut Address<'a, 'hir>>> {
        Any::try_alloc_addr(self)
    }

    #[inline]
    fn try_as_addr(&self) -> compile::Result<Option<&Address<'a, 'hir>>> {
        Any::try_as_addr(self)
    }
}

impl<'a, 'hir> Needs<'a, 'hir> for Address<'a, 'hir> {
    #[inline]
    fn span(&self) -> &'hir dyn Spanned {
        self.span
    }

    #[inline]
    fn output(&self) -> compile::Result<Output> {
        Ok(Address::output(self))
    }

    #[inline]
    fn assign_addr(
        &mut self,
        cx: &mut Ctxt<'_, 'hir, '_>,
        from: InstAddress,
    ) -> compile::Result<()> {
        Address::assign_addr(self, cx, from)
    }

    #[inline]
    fn alloc_output(&mut self) -> compile::Result<Output> {
        Address::alloc_output(self)
    }

    #[inline]
    fn try_alloc_output(&mut self) -> compile::Result<Option<Output>> {
        Ok(Some(Address::alloc_output(self)?))
    }

    #[inline]
    fn try_alloc_addr(&mut self) -> compile::Result<Option<&mut Address<'a, 'hir>>> {
        Ok(Some(Address::alloc_addr(self)?))
    }

    #[inline]
    fn try_as_addr(&self) -> compile::Result<Option<&Address<'a, 'hir>>> {
        Ok(Some(self))
    }
}

#[derive(Clone, Copy)]
pub(super) enum AddressKind {
    /// The value is locally allocated and should be freed in the immediate
    /// scope.
    Local,
    /// The slot has been reserved, but has not been assigned to anything yet.
    Dangling,
    /// The address is assigned from elsewhere and *should not* be touched.
    Assigned,
    /// The address is allocated on behalf of the given scope, and we should
    /// defer deallocating it until the given scope is deallocated.
    Scope(ScopeId),
    /// The address has been freed.
    Freed,
}

impl fmt::Display for AddressKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressKind::Local => write!(f, "local"),
            AddressKind::Dangling => write!(f, "dangling"),
            AddressKind::Assigned => write!(f, "assigned"),
            AddressKind::Scope(scope) => write!(f, "scope({scope})"),
            AddressKind::Freed => write!(f, "freed"),
        }
    }
}

#[must_use = "An allocated address must be freed"]
pub(super) struct Address<'a, 'hir> {
    span: &'hir dyn Spanned,
    scopes: &'a Scopes<'hir>,
    address: InstAddress,
    kind: AddressKind,
    /// A diagnostical name for the address.
    name: Option<&'static str>,
    backtrace: Backtrace,
}

impl<'a, 'hir> Address<'a, 'hir> {
    /// A locally allocated address.
    #[inline]
    #[track_caller]
    pub(super) fn local(
        span: &'hir dyn Spanned,
        scopes: &'a Scopes<'hir>,
        addr: InstAddress,
    ) -> Self {
        Self {
            span,
            scopes,
            address: addr,
            kind: AddressKind::Local,
            name: None,
            backtrace: Backtrace::capture(),
        }
    }

    /// A locally assigned address.
    #[inline]
    #[track_caller]
    pub(super) fn assigned(
        span: &'hir dyn Spanned,
        scopes: &'a Scopes<'hir>,
        addr: InstAddress,
    ) -> Self {
        Self {
            span,
            scopes,
            address: addr,
            kind: AddressKind::Assigned,
            name: None,
            backtrace: Backtrace::capture(),
        }
    }

    /// A locally reserved address.
    #[inline]
    #[track_caller]
    pub(super) fn dangling(
        span: &'hir dyn Spanned,
        scopes: &'a Scopes<'hir>,
        addr: InstAddress,
    ) -> Self {
        Self {
            span,
            scopes,
            address: addr,
            kind: AddressKind::Dangling,
            name: None,
            backtrace: Backtrace::capture(),
        }
    }

    /// Assign a name to the address.
    #[inline]
    pub(super) fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    #[inline]
    pub(super) fn addr(&self) -> InstAddress {
        self.address
    }

    #[inline]
    pub(super) fn alloc_addr(&mut self) -> compile::Result<&mut Self> {
        if matches!(self.kind, AddressKind::Dangling) {
            self.kind = AddressKind::Local;
        }

        Ok(self)
    }

    #[inline]
    pub(super) fn alloc_output(&mut self) -> compile::Result<Output> {
        Ok(self.alloc_addr()?.output())
    }

    #[inline]
    pub(super) fn output(&self) -> Output {
        self.address.output()
    }

    pub(super) fn assign_addr(
        &self,
        cx: &mut Ctxt<'_, '_, '_>,
        from: InstAddress,
    ) -> compile::Result<()> {
        if from != self.address {
            cx.asm.push(
                Inst::Copy {
                    addr: from,
                    out: self.address.output(),
                },
                self.span,
            )?;
        }

        Ok(())
    }

    /// Forget the current address.
    ///
    /// This will be freed when the scope it's associated with is freed.
    pub(super) fn forget(mut self) -> compile::Result<()> {
        self.kind = AddressKind::Freed;
        Ok(())
    }

    pub(super) fn free(self) -> compile::Result<()> {
        self.free_inner(true)
    }

    pub(super) fn free_non_dangling(self) -> compile::Result<()> {
        self.free_inner(false)
    }

    /// Free the current address.
    fn free_inner(mut self, dangling: bool) -> compile::Result<()> {
        match replace(&mut self.kind, AddressKind::Freed) {
            AddressKind::Local | AddressKind::Dangling => {
                self.scopes
                    .free_addr(self.span, self.address, self.name, dangling)?;
            }
            AddressKind::Scope(scope) => {
                if self.scopes.top_id() == scope {
                    self.scopes
                        .free_addr(self.span, self.address, self.name, dangling)?;
                }
            }
            AddressKind::Freed => {
                return Err(compile::Error::msg(
                    self.span,
                    "Address has already been freed",
                ));
            }
            _ => {}
        }

        Ok(())
    }
}

impl fmt::Display for Address<'_, '_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} address {}",
            self.kind,
            DisplayNamed::new(self.address, self.name)
        )
    }
}

impl fmt::Debug for Address<'_, '_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Drop for Address<'_, '_> {
    fn drop(&mut self) {
        if matches!(self.kind, AddressKind::Freed) {
            return;
        }

        rune_diagnose!("{self} was not freed:\nallocated at:\n{}", self.backtrace);
    }
}

/// The kind of a needs.
enum AnyKind<'a, 'hir> {
    Defer {
        scopes: &'a Scopes<'hir>,
        scope: ScopeId,
        name: Option<&'static str>,
    },
    Address {
        address: Address<'a, 'hir>,
    },
    Ignore {
        #[allow(unused)]
        name: Option<&'static str>,
    },
    Freed,
}

impl fmt::Display for AnyKind<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyKind::Defer { scope, name, .. } => {
                write!(f, "defer({})", DisplayNamed::new(scope, *name))
            }
            AnyKind::Address { address } => address.fmt(f),
            AnyKind::Ignore { name } => DisplayNamed::new("ignore", *name).fmt(f),
            AnyKind::Freed => write!(f, "freed"),
        }
    }
}

impl fmt::Debug for AnyKind<'_, '_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// A needs hint for an expression.
/// This is used to contextually determine what an expression is expected to
/// produce.
#[must_use = "A need must be freed when its no longer in use"]
pub(super) struct Any<'a, 'hir> {
    span: &'hir dyn Spanned,
    kind: AnyKind<'a, 'hir>,
    backtrace: Backtrace,
}

impl<'a, 'hir> Any<'a, 'hir> {
    /// A needs that should be ignored.
    #[track_caller]
    pub(super) fn ignore(span: &'hir dyn Spanned) -> Self {
        Self {
            span,
            kind: AnyKind::Ignore { name: None },
            backtrace: Backtrace::capture(),
        }
    }

    /// Defer allocation of a slot until it is requested.
    #[track_caller]
    pub(super) fn defer(scopes: &'a Scopes<'hir>, scope: ScopeId, span: &'hir dyn Spanned) -> Self {
        Self {
            span,
            kind: AnyKind::Defer {
                scopes,
                scope,
                name: None,
            },
            backtrace: Backtrace::capture(),
        }
    }

    /// An assigned address.
    #[track_caller]
    pub(super) fn assigned(
        span: &'hir dyn Spanned,
        scopes: &'a Scopes<'hir>,
        addr: InstAddress,
    ) -> Self {
        Self {
            span,
            kind: AnyKind::Address {
                address: Address::assigned(span, scopes, addr),
            },
            backtrace: Backtrace::capture(),
        }
    }

    /// Assign a name to the request.
    pub(super) fn with_name(mut self, new_name: &'static str) -> Self {
        match &mut self.kind {
            AnyKind::Defer { name, .. } => {
                *name = Some(new_name);
            }
            AnyKind::Address { address } => {
                address.name = Some(new_name);
            }
            AnyKind::Ignore { name, .. } => {
                *name = Some(new_name);
            }
            AnyKind::Freed => {}
        };

        self
    }

    pub(super) fn assign_addr(
        &mut self,
        cx: &mut Ctxt<'_, 'hir, '_>,
        from: InstAddress,
    ) -> compile::Result<()> {
        match &self.kind {
            AnyKind::Defer { scopes, name, .. } => {
                self.kind = AnyKind::Address {
                    address: Address {
                        span: self.span,
                        scopes,
                        address: from,
                        kind: AddressKind::Assigned,
                        name: *name,
                        backtrace: Backtrace::capture(),
                    },
                };
            }
            AnyKind::Address { address } => {
                address.assign_addr(cx, from)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Allocate an address even if it's a locally allocated one.
    #[inline]
    pub(super) fn try_alloc_addr(&mut self) -> compile::Result<Option<&mut Address<'a, 'hir>>> {
        if let AnyKind::Defer {
            scopes,
            scope,
            name,
        } = self.kind
        {
            let address = Address {
                span: self.span,
                scopes,
                address: scopes.alloc_in(self.span, scope)?,
                kind: AddressKind::Scope(scope),
                name,
                backtrace: Backtrace::capture(),
            };

            self.kind = AnyKind::Address { address };
        }

        match &mut self.kind {
            AnyKind::Address { address } => Ok(Some(address.alloc_addr()?)),
            _ => Ok(None),
        }
    }

    /// Get the needs as an output.
    #[inline]
    pub(super) fn try_alloc_output(&mut self) -> compile::Result<Option<Output>> {
        let Some(addr) = self.try_alloc_addr()? else {
            return Ok(None);
        };

        Ok(Some(addr.output()))
    }

    /// Test if any sort of value is needed.
    #[inline(always)]
    pub(super) fn alloc_output(&mut self) -> compile::Result<Output> {
        let Some(addr) = self.try_alloc_addr()? else {
            return Ok(Output::discard());
        };

        Ok(addr.output())
    }

    /// Try to treat as an address.
    #[inline]
    pub(super) fn addr(&self) -> compile::Result<&Address<'a, 'hir>> {
        match &self.kind {
            AnyKind::Address { address } => Ok(address),
            kind => Err(compile::Error::msg(
                self.span,
                format!("No address for need {kind}"),
            )),
        }
    }

    /// Coerce into an owned address.
    #[inline]
    pub(super) fn into_addr(mut self) -> compile::Result<Address<'a, 'hir>> {
        match replace(&mut self.kind, AnyKind::Freed) {
            AnyKind::Address { address } => Ok(address),
            kind => Err(compile::Error::msg(
                self.span,
                format!("No address for need {kind}"),
            )),
        }
    }

    /// Coerce into a output.
    #[inline]
    pub(super) fn output(&self) -> compile::Result<Output> {
        match &self.kind {
            AnyKind::Address { address } => Ok(Output::keep(address.address.offset())),
            AnyKind::Ignore { .. } => Ok(Output::discard()),
            kind => Err(compile::Error::msg(
                self.span,
                format!("Needs {kind} has not been allocated for output"),
            )),
        }
    }

    /// Gets the initialized address unless it is specifically set to `None`.
    #[inline]
    pub(super) fn try_as_addr(&self) -> compile::Result<Option<&Address<'a, 'hir>>> {
        match &self.kind {
            AnyKind::Address { address } => {
                if matches!(address.kind, AddressKind::Dangling) {
                    return Err(compile::Error::msg(
                        self.span,
                        "Expected address to be initialized",
                    ));
                }

                Ok(Some(address))
            }
            _ => Ok(None),
        }
    }

    /// Free the current needs.
    pub(super) fn free(mut self) -> compile::Result<()> {
        if let AnyKind::Address { address } = replace(&mut self.kind, AnyKind::Freed) {
            address.free()?;
        }

        Ok(())
    }
}

impl fmt::Display for Any<'_, '_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl fmt::Debug for Any<'_, '_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Drop for Any<'_, '_> {
    fn drop(&mut self) {
        if matches!(self.kind, AnyKind::Ignore { .. } | AnyKind::Freed) {
            return;
        }

        rune_diagnose!("{self} was not freed:\nallocated at:\n{}", self.backtrace);
    }
}
