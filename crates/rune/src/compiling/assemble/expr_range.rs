use crate::compiling::assemble::prelude::*;

/// Compile a range expression.
impl Assemble for ast::ExprRange {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprRange => {:?}", c.source.source(span));

        let guard = c.scopes.push_child(span)?;

        if needs.value() {
            let from = if let Some(from) = &self.from {
                from.assemble(c, needs)?;
                c.asm.push(
                    Inst::Variant {
                        variant: InstVariant::Some,
                    },
                    from.span(),
                );
                from.span()
            } else {
                c.asm.push(
                    Inst::Variant {
                        variant: InstVariant::None,
                    },
                    span,
                );
                span
            };

            c.scopes.decl_anon(from)?;

            let to = if let Some(to) = &self.to {
                to.assemble(c, needs)?;
                c.asm.push(
                    Inst::Variant {
                        variant: InstVariant::Some,
                    },
                    to.span(),
                );
                to.span()
            } else {
                c.asm.push(
                    Inst::Variant {
                        variant: InstVariant::None,
                    },
                    span,
                );
                span
            };

            c.scopes.decl_anon(to)?;

            let limits = match &self.limits {
                ast::ExprRangeLimits::HalfOpen(..) => InstRangeLimits::HalfOpen,
                ast::ExprRangeLimits::Closed(..) => InstRangeLimits::Closed,
            };

            c.asm.push(Inst::Range { limits }, span);
            c.scopes.undecl_anon(span, 2)?;
        } else {
            if let Some(from) = &self.from {
                from.assemble(c, needs)?;
            }

            if let Some(to) = &self.to {
                to.assemble(c, needs)?;
            }
        }

        c.scopes.pop(guard, span)?;
        Ok(())
    }
}
