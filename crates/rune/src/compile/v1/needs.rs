use core::fmt;

use crate::ast::Spanned;
use crate::compile;
use crate::runtime::{Inst, InstAddress, Output};

use super::{Ctxt, ScopeId, Scopes};

#[derive(Debug, Clone, Copy)]
pub(super) enum NeedsAddressKind {
    /// The value is locally allocated and should be freed in the immediate scope.
    Local,
    /// The address is assigned from elsewhere and *should not* be touched.
    Assigned,
    /// The address is allocated on behalf of the given scope, and we should
    /// defer deallocating it until the given scope is deallocated.
    Scope(ScopeId),
}

#[derive(Clone, Copy)]
pub(super) struct NeedsAddress<'hir> {
    pub(super) span: &'hir dyn Spanned,
    pub(super) addr: InstAddress,
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

    #[inline]
    pub(super) fn addr(&self) -> InstAddress {
        self.addr
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
    Alloc(ScopeId),
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
            kind: NeedsKind::Alloc(scope),
        })
    }

    /// Allocate on demand.
    pub(super) fn alloc(
        cx: &mut Ctxt<'_, 'hir, '_>,
        span: &'hir dyn Spanned,
    ) -> compile::Result<Self> {
        let Some(scope) = cx.scopes.top_id() else {
            return Err(compile::Error::msg(span, "Expected top scope"));
        };

        Ok(Self {
            span,
            kind: NeedsKind::Alloc(scope),
        })
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
            NeedsKind::Alloc(..) => {
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
        matches!(self.kind, NeedsKind::Alloc(..) | NeedsKind::Address(..))
    }

    /// Coerce into a value.
    #[inline]
    pub(super) fn try_alloc_addr(
        &mut self,
        scopes: &mut Scopes<'_>,
    ) -> compile::Result<Option<InstAddress>> {
        match &self.kind {
            NeedsKind::Alloc(scope) => {
                let addr = scopes.alloc_in(self.span, *scope)?;

                self.kind = NeedsKind::Address(NeedsAddress {
                    span: self.span,
                    addr,
                    kind: NeedsAddressKind::Scope(*scope),
                });

                Ok(Some(addr))
            }
            NeedsKind::Address(addr) => Ok(Some(addr.addr)),
            NeedsKind::None => Ok(None),
        }
    }

    /// Test if any sort of value is needed.
    #[inline(always)]
    pub(super) fn alloc_output(&mut self, scopes: &mut Scopes<'_>) -> compile::Result<Output> {
        let Some(addr) = self.try_alloc_addr(scopes)? else {
            return Ok(Output::discard());
        };

        Ok(addr.output())
    }

    /// Coerce into a value.
    #[inline]
    #[deprecated = "Use as_addr instead to check that the address has been initialized"]
    pub(super) fn addr(&self) -> compile::Result<InstAddress> {
        if let NeedsKind::Address(addr) = &self.kind {
            return Ok(addr.addr);
        };

        Err(compile::Error::msg(
            self.span,
            "Address has not been initialized",
        ))
    }

    /// Coerce into a output.
    #[inline]
    pub(super) fn output(&self) -> compile::Result<Output> {
        match &self.kind {
            NeedsKind::Alloc(..) => Err(compile::Error::msg(
                self.span,
                "Needs has not been initialized for output",
            )),
            NeedsKind::Address(addr) => Ok(Output::keep(addr.addr.offset())),
            NeedsKind::None => Ok(Output::discard()),
        }
    }

    /// Get the needs as an address.
    #[inline]
    pub(super) fn as_addr(&self) -> Option<&NeedsAddress<'hir>> {
        match &self.kind {
            NeedsKind::Address(addr) => Some(addr),
            _ => None,
        }
    }

    /// Get the needs as an output.
    #[inline]
    pub(super) fn as_output(&self) -> Option<Output> {
        match &self.kind {
            NeedsKind::Address(addr) => Some(addr.output()),
            _ => None,
        }
    }

    /// Free the current needs.
    pub(super) fn free(self, scopes: &mut Scopes<'hir>) -> compile::Result<()> {
        if let NeedsKind::Address(addr) = self.kind {
            scopes.free(addr)?;
        }

        Ok(())
    }
}
