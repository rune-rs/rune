use crate::compiling::v1::assemble::prelude::*;

macro_rules! tuple {
    ($slf:expr, $variant:ident, $c:ident, $span:expr, $($var:ident),*) => {{
        let guard = $c.scopes.push_child($span)?;

        let mut it = $slf.items.iter();

        $(
        let ($var, _) = it.next().ok_or_else(|| CompileError::new($span, CompileErrorKind::Custom { message: "items ended unexpectedly" }))?;
        let $var = $var.assemble($c, Needs::Value)?.apply_targeted($c)?;
        )*

        $c.asm.push(
            Inst::$variant {
                args: [$($var,)*],
            },
            $span,
        );

        $c.scopes.pop(guard, $span)?;
    }};
}

/// Compile a literal tuple.
impl Assemble for ast::ExprTuple {
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprTuple => {:?}", c.q.sources.source(c.source_id, span));

        if self.items.is_empty() {
            c.asm.push(Inst::unit(), span);
        } else {
            match self.items.len() {
                1 => tuple!(self, Tuple1, c, span, e1),
                2 => tuple!(self, Tuple2, c, span, e1, e2),
                3 => tuple!(self, Tuple3, c, span, e1, e2, e3),
                4 => tuple!(self, Tuple4, c, span, e1, e2, e3, e4),
                _ => {
                    for (expr, _) in &self.items {
                        expr.assemble(c, Needs::Value)?.apply(c)?;
                        c.scopes.decl_anon(expr.span())?;
                    }

                    c.asm.push(
                        Inst::Tuple {
                            count: self.items.len(),
                        },
                        span,
                    );

                    c.scopes.undecl_anon(span, self.items.len())?;
                }
            }
        }

        if !needs.value() {
            c.diagnostics.not_used(c.source_id, span, c.context());
            c.asm.push(Inst::Pop, span);
        }

        Ok(Asm::top(span))
    }
}
