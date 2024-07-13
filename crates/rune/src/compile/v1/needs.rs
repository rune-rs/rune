use core::fmt;

use crate::ast::Spanned;
use crate::compile::{self, Assembly};
use crate::runtime::{Inst, InstAddress, Output};

use super::{Ctxt, ScopeId, Scopes};

#[derive(Debug, Clone, Copy)]
pub(super) enum NeedsAddressKind {
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
}

#[derive(Clone, Copy)]
pub(super) struct NeedsAddress<'hir> {
    pub(super) span: &'hir dyn Spanned,
    addr: InstAddress,
    pub(super) kind: NeedsAddressKind,
}

impl<'hir> NeedsAddress<'hir> {
    /// Construct an empty address.
    pub(super) const fn empty(span: &'hir dyn Spanned) -> Self {
        Self {
            span,
            addr: InstAddress::ZERO,
            kind: NeedsAddressKind::Assigned,
        }
    }

    /// A locally allocated address.
    #[inline]
    pub(super) fn with_local(span: &'hir dyn Spanned, addr: InstAddress) -> Self {
        Self {
            span,
            addr,
            kind: NeedsAddressKind::Local,
        }
    }

    /// A locally reserved address.
    #[inline]
    pub(super) fn with_reserved(span: &'hir dyn Spanned, addr: InstAddress) -> Self {
        Self {
            span,
            addr,
            kind: NeedsAddressKind::Dangling,
        }
    }

    #[inline]
    pub(super) fn addr(&self) -> InstAddress {
        self.addr
    }

    #[inline]
    pub(super) fn alloc_addr(&mut self) -> compile::Result<InstAddress> {
        if matches!(self.kind, NeedsAddressKind::Dangling) {
            self.kind = NeedsAddressKind::Local;
        }

        Ok(self.addr)
    }

    #[inline]
    pub(super) fn alloc_output(&mut self) -> compile::Result<Output> {
        Ok(self.alloc_addr()?.output())
    }

    #[inline]
    pub(super) fn output(&self) -> Output {
        self.addr.output()
    }

    pub(super) fn assign_addr(
        &self,
        cx: &mut Ctxt<'_, '_, '_>,
        from: InstAddress,
    ) -> compile::Result<()> {
        if from != self.addr {
            cx.asm.push(
                Inst::Copy {
                    addr: from,
                    out: self.addr.output(),
                },
                self.span,
            )?;
        }

        Ok(())
    }
}

impl fmt::Debug for NeedsAddress<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NeedsAddress")
            .field("span", &self.span.span())
            .field("addr", &self.addr)
            .field("kind", &self.kind)
            .finish()
    }
}

/// The kind of a needs.
#[derive(Debug)]
pub(super) enum NeedsKind<'hir> {
    Alloc { scope: ScopeId },
    Address(NeedsAddress<'hir>),
    None,
}

/// A needs hint for an expression.
/// This is used to contextually determine what an expression is expected to
/// produce.
pub(super) struct Needs<'hir> {
    pub(super) span: &'hir dyn Spanned,
    pub(super) kind: NeedsKind<'hir>,
}

impl fmt::Debug for Needs<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Needs")
            .field("span", &self.span.span())
            .field("kind", &self.kind)
            .finish()
    }
}

impl<'hir> Needs<'hir> {
    /// A none needs.
    pub(super) fn none(span: &'hir dyn Spanned) -> Self {
        Self {
            span,
            kind: NeedsKind::None,
        }
    }

    /// Allocate on demand inside of a specific scope.
    pub(super) fn alloc_in(scope: ScopeId, span: &'hir dyn Spanned) -> compile::Result<Self> {
        Ok(Self {
            span,
            kind: NeedsKind::Alloc { scope },
        })
    }

    /// Allocate on demand.
    pub(super) fn alloc(cx: &mut Ctxt<'_, 'hir, '_>, span: &'hir dyn Spanned) -> Self {
        Self {
            span,
            kind: NeedsKind::Alloc {
                scope: cx.scopes.top_id(),
            },
        }
    }

    /// A provided address.
    pub(super) fn with_local(span: &'hir dyn Spanned, addr: InstAddress) -> Self {
        Self {
            span,
            kind: NeedsKind::Address(NeedsAddress {
                span,
                addr,
                kind: NeedsAddressKind::Local,
            }),
        }
    }

    /// An assigned address.
    pub(super) fn with_assigned(span: &'hir dyn Spanned, addr: InstAddress) -> Self {
        Self {
            span,
            kind: NeedsKind::Address(NeedsAddress {
                span,
                addr,
                kind: NeedsAddressKind::Assigned,
            }),
        }
    }

    pub(super) fn assign_addr(
        &mut self,
        cx: &mut Ctxt<'_, 'hir, '_>,
        from: InstAddress,
    ) -> compile::Result<()> {
        match &self.kind {
            NeedsKind::Alloc { .. } => {
                self.kind = NeedsKind::Address(NeedsAddress {
                    span: self.span,
                    addr: from,
                    kind: NeedsAddressKind::Assigned,
                });
            }
            NeedsKind::Address(addr) => {
                addr.assign_addr(cx, from)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Test if any sort of value is needed.
    #[inline(always)]
    pub(super) fn value(&self) -> bool {
        matches!(self.kind, NeedsKind::Alloc { .. } | NeedsKind::Address(..))
    }

    /// Test if any sort of value is needed.
    #[inline(always)]
    pub(super) fn alloc_output(&mut self, scopes: &mut Scopes<'_>) -> compile::Result<Output> {
        let Some(addr) = self.try_alloc_addr(scopes)? else {
            return Ok(Output::discard());
        };

        Ok(addr.output())
    }

    /// Get the needs as an output.
    #[inline]
    pub(super) fn try_alloc_output(
        &mut self,
        scopes: &mut Scopes<'hir>,
    ) -> compile::Result<Option<Output>> {
        let Some(addr) = self.try_alloc_addr(scopes)? else {
            return Ok(None);
        };

        Ok(Some(addr.output()))
    }

    /// Coerce into a value.
    #[inline]
    pub(super) fn try_alloc_addr(
        &mut self,
        scopes: &mut Scopes<'_>,
    ) -> compile::Result<Option<InstAddress>> {
        match &mut self.kind {
            NeedsKind::Alloc { scope } => {
                let addr = scopes.alloc_in(self.span, *scope)?;

                self.kind = NeedsKind::Address(NeedsAddress {
                    span: self.span,
                    addr,
                    kind: NeedsAddressKind::Scope(*scope),
                });

                Ok(Some(addr))
            }
            NeedsKind::Address(addr) => Ok(Some(addr.alloc_addr()?)),
            NeedsKind::None { .. } => Ok(None),
        }
    }

    /// Coerce into a output.
    #[inline]
    pub(super) fn output(&self) -> compile::Result<Output> {
        match &self.kind {
            NeedsKind::Alloc { .. } => Err(compile::Error::msg(
                self.span,
                "Needs has not been initialized for output",
            )),
            NeedsKind::Address(addr) => Ok(Output::keep(addr.addr.offset())),
            NeedsKind::None { .. } => Ok(Output::discard()),
        }
    }

    /// Get the needs as an address.
    #[inline]
    pub(super) fn as_addr(&self) -> Option<&NeedsAddress<'hir>> {
        match &self.kind {
            NeedsKind::Address(addr) => {
                if matches!(addr.kind, NeedsAddressKind::Dangling) {
                    return None;
                }

                Some(addr)
            }
            _ => None,
        }
    }

    /// Free the current needs.
    pub(super) fn free(self, asm: &mut Assembly, scopes: &mut Scopes<'hir>) -> compile::Result<()> {
        if let NeedsKind::Address(addr) = self.kind {
            scopes.free(asm, addr)?;
        }

        Ok(())
    }
}
