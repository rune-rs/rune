use crate::compiling::v1::assemble::prelude::*;
use crate::query::BuiltInTemplate;

/// Compile a literal template string.
impl Assemble for BuiltInTemplate {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span;
        log::trace!("BuiltInTemplate => {:?}", c.source.source(span));

        let expected = c.scopes.push_child(span)?;
        let mut size_hint = 0;
        let mut expansions = 0;

        for expr in &self.exprs {
            if let ast::Expr::Lit(expr_lit) = expr {
                if let ast::ExprLit {
                    lit: ast::Lit::Str(s),
                    ..
                } = &**expr_lit
                {
                    let s = s.resolve_template_string(c.storage, &c.source)?;
                    size_hint += s.len();

                    let slot = c.unit.new_static_string(span, &s)?;
                    c.asm.push(Inst::String { slot }, span);
                    c.scopes.decl_anon(span)?;
                    continue;
                }
            }

            expansions += 1;
            expr.assemble(c, Needs::Value)?.apply(c)?;
            c.scopes.decl_anon(span)?;
        }

        if self.from_literal && expansions == 0 {
            c.diagnostics
                .template_without_expansions(c.source_id, span, c.context());
        }

        c.asm.push(
            Inst::StringConcat {
                len: self.exprs.len(),
                size_hint,
            },
            span,
        );

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        let _ = c.scopes.pop(expected, span)?;
        Ok(Asm::top(span))
    }
}
