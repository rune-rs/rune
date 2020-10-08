use crate::compiling::compile::prelude::*;

/// Compile an expression.
impl Compile2 for ast::Expr {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Expr => {:?}", c.source.source(span));

        if let Some(span) = self.attributes().option_span() {
            return Err(CompileError::internal(span, "attributes are not supported"));
        }

        match self {
            ast::Expr::Path(path) => {
                path.compile2(c, needs)?;
            }
            ast::Expr::ExprWhile(expr_while) => {
                expr_while.compile2(c, needs)?;
            }
            ast::Expr::ExprFor(expr_for) => {
                expr_for.compile2(c, needs)?;
            }
            ast::Expr::ExprLoop(expr_loop) => {
                expr_loop.compile2(c, needs)?;
            }
            ast::Expr::ExprLet(expr_let) => {
                expr_let.compile2(c, needs)?;
            }
            ast::Expr::ExprGroup(expr) => {
                expr.expr.compile2(c, needs)?;
            }
            ast::Expr::ExprUnary(expr_unary) => {
                expr_unary.compile2(c, needs)?;
            }
            ast::Expr::ExprAssign(expr_assign) => {
                expr_assign.compile2(c, needs)?;
            }
            ast::Expr::ExprBinary(expr_binary) => {
                expr_binary.compile2(c, needs)?;
            }
            ast::Expr::ExprIf(expr_if) => {
                expr_if.compile2(c, needs)?;
            }
            ast::Expr::ExprIndex(expr_index_get) => {
                expr_index_get.compile2(c, needs)?;
            }
            ast::Expr::ExprBreak(expr_break) => {
                expr_break.compile2(c, needs)?;
            }
            ast::Expr::ExprYield(expr_yield) => {
                expr_yield.compile2(c, needs)?;
            }
            ast::Expr::ExprBlock(expr_block) => {
                expr_block.compile2(c, needs)?;
            }
            ast::Expr::ExprReturn(expr_return) => {
                expr_return.compile2(c, needs)?;
            }
            ast::Expr::ExprMatch(expr_match) => {
                expr_match.compile2(c, needs)?;
            }
            ast::Expr::ExprAwait(expr_await) => {
                expr_await.compile2(c, needs)?;
            }
            ast::Expr::ExprTry(expr_try) => {
                expr_try.compile2(c, needs)?;
            }
            ast::Expr::ExprSelect(expr_select) => {
                expr_select.compile2(c, needs)?;
            }
            ast::Expr::ExprCall(expr_call) => {
                expr_call.compile2(c, needs)?;
            }
            ast::Expr::ExprFieldAccess(expr_field_access) => {
                expr_field_access.compile2(c, needs)?;
            }
            ast::Expr::ExprClosure(expr_closure) => {
                expr_closure.compile2(c, needs)?;
            }
            ast::Expr::ExprLit(expr_lit) => {
                expr_lit.lit.compile2(c, needs)?;
            }
            ast::Expr::MacroCall(expr_call_macro) => {
                return Err(CompileError::internal(
                    expr_call_macro,
                    "encountered unexpanded macro",
                ));
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
