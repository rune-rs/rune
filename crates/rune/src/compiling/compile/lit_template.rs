use crate::compiling::compile::prelude::*;

/// Compile a literal template string.
impl Compile<(&ast::LitTemplate, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_template, needs): (&ast::LitTemplate, Needs)) -> CompileResult<()> {
        let span = lit_template.span();
        log::trace!("LitTemplate => {:?}", self.source.source(span));

        let expected = self.scopes.push_child(span)?;
        let mut size_hint = 0;
        let mut expansions = 0;

        for (expr, _) in &lit_template.args {
            if let ast::Expr::ExprLit(expr_lit) = expr {
                if let ast::ExprLit {
                    lit: ast::Lit::Str(s),
                    ..
                } = &**expr_lit
                {
                    let s = s.resolve_template_string(&self.storage, &self.source)?;
                    size_hint += s.len();

                    let slot = self.unit.new_static_string(span, &s)?;
                    self.asm.push(Inst::String { slot }, span);
                    self.scopes.decl_anon(span)?;
                    continue;
                }
            }

            expansions += 1;
            self.compile((expr, Needs::Value))?;
            self.scopes.decl_anon(span)?;
        }

        if expansions == 0 {
            self.warnings
                .template_without_expansions(self.source_id, span, self.context());
        }

        self.asm.push(
            Inst::StringConcat {
                len: lit_template.args.len(),
                size_hint,
            },
            span,
        );

        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        let _ = self.scopes.pop(expected, span)?;
        Ok(())
    }
}
