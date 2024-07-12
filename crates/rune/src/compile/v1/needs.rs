use core::fmt;

use crate::ast::Spanned;
use crate::compile;
use crate::runtime::{Inst, InstAddress, Output};

use super::{Ctxt, ScopeId, Scopes};

#[derive(Debug)]
pub(super) enum NeedsAddressKind {
    /// The value is locally allocated and should be freed in the immediate scope.
    Local,
    /// The address is assigned from elsewhere and *should not* be touched.
    Assigned,
    /// The address is allocated on behalf of the given scope, and we should
    /// defer deallocating it until the given scope is deallocated.
    Scope(ScopeId),
}

#[derive(Debug)]
pub(super) struct NeedsAddress {
    pub(super) addr: InstAddress,
    pub(super) kind: NeedsAddressKind,
}

/// The kind of a needs.
#[derive(Debug)]
pub(super) enum NeedsKind {
    Alloc(ScopeId),
    Address(NeedsAddress),
    None,
}

/// A needs hint for an expression.
/// This is used to contextually determine what an expression is expected to
/// produce.
pub(super) struct Needs<'a> {
    pub(super) span: &'a dyn Spanned,
    pub(super) kind: NeedsKind,
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

impl<'a> Needs<'a> {
    /// A none needs.
    pub(super) fn none(span: &'a dyn Spanned) -> Self {
        Self {
            span,
            kind: NeedsKind::None,
        }
    }

    /// Allocate on demand inside of a specific scope.
    pub(super) fn alloc_in(scope: ScopeId, span: &'a dyn Spanned) -> compile::Result<Self> {
        Ok(Self {
            span,
            kind: NeedsKind::Alloc(scope),
        })
    }

    /// Allocate on demand.
    pub(super) fn alloc(cx: &mut Ctxt<'_, '_, '_>, span: &'a dyn Spanned) -> compile::Result<Self> {
        let Some(scope) = cx.scopes.top_id() else {
            return Err(compile::Error::msg(span, "Expected top scope"));
        };

        Ok(Self {
            span,
            kind: NeedsKind::Alloc(scope),
        })
    }

    /// A provided address.
    pub(super) fn with_addr(span: &'a dyn Spanned, addr: InstAddress) -> Self {
        Self {
            span,
            kind: NeedsKind::Address(NeedsAddress {
                addr,
                kind: NeedsAddressKind::Local,
            }),
        }
    }

    pub(super) fn assign_addr(
        &mut self,
        cx: &mut Ctxt<'_, '_, '_>,
        from: InstAddress,
    ) -> compile::Result<()> {
        match &self.kind {
            NeedsKind::Alloc(..) => {
                self.kind = NeedsKind::Address(NeedsAddress {
                    addr: from,
                    kind: NeedsAddressKind::Assigned,
                });
            }
            NeedsKind::Address(addr) => {
                cx.asm.push(
                    Inst::Copy {
                        addr: from,
                        out: addr.addr.output(),
                    },
                    self.span,
                )?;
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
}
