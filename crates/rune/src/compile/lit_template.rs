use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::{Compile, Resolve as _};
use runestick::Inst;

/// Compile a literal template string.
impl Compile<(&ast::LitTemplate, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_template, needs): (&ast::LitTemplate, Needs)) -> CompileResult<()> {
        let span = lit_template.span();
        log::trace!("LitTemplate => {:?}", self.source.source(span));

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
            return Ok(());
        }

        let template = lit_template.resolve(&self.storage, &*self.source)?;

        if !template.has_expansions {
            self.warnings
                .template_without_expansions(self.source_id, span, self.context());
        }

        let scope = self.scopes.child(span)?;
        let expected = self.scopes.push(scope);

        for c in template.components.iter() {
            match c {
                ast::TemplateComponent::String(string) => {
                    let slot = self.unit.borrow_mut().new_static_string(&string)?;
                    self.asm.push(Inst::String { slot }, span);
                    self.scopes.decl_anon(span)?;
                }
                ast::TemplateComponent::Expr(expr) => {
                    self.compile((&**expr, Needs::Value))?;
                    self.scopes.decl_anon(span)?;
                }
            }
        }

        self.asm.push(
            Inst::StringConcat {
                len: template.components.len(),
                size_hint: template.size_hint,
            },
            span,
        );

        let _ = self.scopes.pop(expected, span)?;
        Ok(())
    }
}
