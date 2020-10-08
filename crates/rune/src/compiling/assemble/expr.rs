use crate::compiling::assemble::prelude::*;
use crate::query::BuiltInMacro;

/// Compile an expression.
impl Assemble for ast::Expr {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Expr => {:?}", c.source.source(span));

        match self {
            ast::Expr::Path(path) => {
                path.assemble(c, needs)?;
            }
            ast::Expr::ExprWhile(expr_while) => {
                expr_while.assemble(c, needs)?;
            }
            ast::Expr::ExprFor(expr_for) => {
                expr_for.assemble(c, needs)?;
            }
            ast::Expr::ExprLoop(expr_loop) => {
                expr_loop.assemble(c, needs)?;
            }
            ast::Expr::ExprLet(expr_let) => {
                expr_let.assemble(c, needs)?;
            }
            ast::Expr::ExprGroup(expr) => {
                expr.expr.assemble(c, needs)?;
            }
            ast::Expr::ExprUnary(expr_unary) => {
                expr_unary.assemble(c, needs)?;
            }
            ast::Expr::ExprAssign(expr_assign) => {
                expr_assign.assemble(c, needs)?;
            }
            ast::Expr::ExprBinary(expr_binary) => {
                expr_binary.assemble(c, needs)?;
            }
            ast::Expr::ExprIf(expr_if) => {
                expr_if.assemble(c, needs)?;
            }
            ast::Expr::ExprIndex(expr_index_get) => {
                expr_index_get.assemble(c, needs)?;
            }
            ast::Expr::ExprBreak(expr_break) => {
                expr_break.assemble(c, needs)?;
            }
            ast::Expr::ExprYield(expr_yield) => {
                expr_yield.assemble(c, needs)?;
            }
            ast::Expr::ExprBlock(expr_block) => {
                expr_block.assemble(c, needs)?;
            }
            ast::Expr::ExprReturn(expr_return) => {
                expr_return.assemble(c, needs)?;
            }
            ast::Expr::ExprMatch(expr_match) => {
                expr_match.assemble(c, needs)?;
            }
            ast::Expr::ExprAwait(expr_await) => {
                expr_await.assemble(c, needs)?;
            }
            ast::Expr::ExprTry(expr_try) => {
                expr_try.assemble(c, needs)?;
            }
            ast::Expr::ExprSelect(expr_select) => {
                expr_select.assemble(c, needs)?;
            }
            ast::Expr::ExprCall(expr_call) => {
                expr_call.assemble(c, needs)?;
            }
            ast::Expr::ExprFieldAccess(expr_field_access) => {
                expr_field_access.assemble(c, needs)?;
            }
            ast::Expr::ExprClosure(expr_closure) => {
                expr_closure.assemble(c, needs)?;
            }
            ast::Expr::ExprLit(expr_lit) => {
                expr_lit.lit.assemble(c, needs)?;
            }
            ast::Expr::MacroCall(expr_call_macro) => {
                let internal_macro = c.query.builtin_macro_for(&**expr_call_macro)?;

                match &*internal_macro {
                    BuiltInMacro::Template(template) => {
                        template.assemble(c, needs)?;
                    }
                    BuiltInMacro::FormatSpec(format_spec) => {
                        format_spec.assemble(c, needs)?;
                    }
                }
            }
            // NB: declarations are not used in this compilation stage.
            // They have been separately indexed and will be built when queried
            // for.
            ast::Expr::Item(decl) => {
                let span = decl.span();

                if needs.value() {
                    c.asm.push(Inst::unit(), span);
                }
            }
        }

        Ok(())
    }
}
