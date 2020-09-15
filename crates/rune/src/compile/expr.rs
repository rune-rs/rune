use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::worker::Expanded;
use crate::CompileResult;
use crate::{CompileError, Spanned as _};
use runestick::Inst;

/// Compile an expression.
impl Compile<(&ast::Expr, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr, needs): (&ast::Expr, Needs)) -> CompileResult<()> {
        let span = expr.span();
        log::trace!("Expr => {:?}", self.source.source(span));

        match expr {
            ast::Expr::Self_(self_) => {
                self.compile((self_, needs))?;
            }
            ast::Expr::Path(path) => {
                self.compile((path, needs))?;
            }
            ast::Expr::ExprWhile(expr_while) => {
                self.compile((expr_while, needs))?;
            }
            ast::Expr::ExprFor(expr_for) => {
                self.compile((expr_for, needs))?;
            }
            ast::Expr::ExprLoop(expr_loop) => {
                self.compile((expr_loop, needs))?;
            }
            ast::Expr::ExprLet(expr_let) => {
                self.compile((expr_let, needs))?;
            }
            ast::Expr::ExprGroup(expr) => {
                self.compile((&*expr.expr, needs))?;
            }
            ast::Expr::ExprUnary(expr_unary) => {
                self.compile((expr_unary, needs))?;
            }
            ast::Expr::ExprBinary(expr_binary) => {
                self.compile((expr_binary, needs))?;
            }
            ast::Expr::ExprIf(expr_if) => {
                self.compile((expr_if, needs))?;
            }
            ast::Expr::ExprIndexSet(expr_index_set) => {
                self.compile((expr_index_set, needs))?;
            }
            ast::Expr::ExprIndexGet(expr_index_get) => {
                self.compile((expr_index_get, needs))?;
            }
            ast::Expr::ExprBreak(expr_break) => {
                self.compile(expr_break)?;
            }
            ast::Expr::ExprYield(expr_yield) => {
                self.compile((expr_yield, needs))?;
            }
            ast::Expr::ExprBlock(expr_block) => {
                self.compile((expr_block, needs))?;
            }
            ast::Expr::ExprAsync(expr_async) => {
                self.compile((expr_async, needs))?;
            }
            ast::Expr::ExprReturn(expr_return) => {
                self.compile((expr_return, needs))?;
            }
            ast::Expr::ExprMatch(expr_match) => {
                self.compile((expr_match, needs))?;
            }
            ast::Expr::ExprAwait(expr_await) => {
                self.compile((expr_await, needs))?;
            }
            ast::Expr::ExprTry(expr_try) => {
                self.compile((expr_try, needs))?;
            }
            ast::Expr::ExprSelect(expr_select) => {
                self.compile((expr_select, needs))?;
            }
            ast::Expr::ExprCall(expr_call) => {
                self.compile((expr_call, needs))?;
            }
            ast::Expr::ExprFieldAccess(expr_field_access) => {
                self.compile((expr_field_access, needs))?;
            }
            ast::Expr::ExprClosure(expr_closure) => {
                self.compile((expr_closure, needs))?;
            }
            ast::Expr::LitUnit(lit_unit) => {
                self.compile((lit_unit, needs))?;
            }
            ast::Expr::LitTuple(lit_tuple) => {
                self.compile((lit_tuple, needs))?;
            }
            ast::Expr::LitBool(lit_bool) => {
                self.compile((lit_bool, needs))?;
            }
            ast::Expr::LitNumber(lit_number) => {
                self.compile((lit_number, needs))?;
            }
            ast::Expr::LitVec(lit_vec) => {
                self.compile((lit_vec, needs))?;
            }
            ast::Expr::LitObject(lit_object) => {
                self.compile((lit_object, needs))?;
            }
            ast::Expr::LitChar(lit_char) => {
                self.compile((lit_char, needs))?;
            }
            ast::Expr::LitStr(lit_str) => {
                self.compile((lit_str, needs))?;
            }
            ast::Expr::LitByte(lit_char) => {
                self.compile((lit_char, needs))?;
            }
            ast::Expr::LitByteStr(lit_str) => {
                self.compile((lit_str, needs))?;
            }
            ast::Expr::LitTemplate(lit_template) => {
                self.compile((lit_template, needs))?;
            }
            ast::Expr::MacroCall(expr_call_macro) => {
                let _guard = self.items.push_macro();
                let item = self.items.item();

                if let Some(Expanded::Expr(expr)) = self.expanded.get(&item) {
                    self.compile((expr, needs))?;
                } else {
                    let span = expr_call_macro.span();

                    return Err(CompileError::internal(span, "macro has not been expanded"));
                }
            }
            // NB: declarations are not used in this compilation stage.
            // They have been separately indexed and will be built when queried
            // for.
            ast::Expr::Item(decl) => {
                let span = decl.span();

                if needs.value() {
                    self.asm.push(Inst::Unit, span);
                }
            }
        }

        Ok(())
    }
}
