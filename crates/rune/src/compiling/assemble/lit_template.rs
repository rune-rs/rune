use crate::compiling::assemble::prelude::*;

/// Compile a literal template string.
impl Assemble for ast::LitTemplate {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("LitTemplate => {:?}", c.source.source(span));

        let expected = c.scopes.push_child(span)?;
        let mut size_hint = 0;
        let mut expansions = 0;

        for (expr, _) in &self.args {
            if let ast::Expr::ExprLit(expr_lit) = expr {
                if let ast::ExprLit {
                    lit: ast::Lit::Str(s),
                    ..
                } = &**expr_lit
                {
                    let s = s.resolve_template_string(&c.storage, &c.source)?;
                    size_hint += s.len();

                    let slot = c.unit.new_static_string(span, &s)?;
                    c.asm.push(Inst::String { slot }, span);
                    c.scopes.decl_anon(span)?;
                    continue;
                }
            }

            expansions += 1;
            expr.assemble(c, Needs::Value)?;
            c.scopes.decl_anon(span)?;
        }

        if expansions == 0 {
            c.warnings
                .template_without_expansions(c.source_id, span, c.context());
        }

        c.asm.push(
            Inst::StringConcat {
                len: self.args.len(),
                size_hint,
            },
            span,
        );

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        let _ = c.scopes.pop(expected, span)?;
        Ok(())
    }
}
