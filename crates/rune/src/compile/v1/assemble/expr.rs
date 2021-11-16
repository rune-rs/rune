use crate::compile::v1::assemble::prelude::*;
use crate::query::BuiltInMacro;

/// Compile an expression.
impl Assemble for ast::Expr {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("Expr => {:?}", c.q.sources.source(c.source_id, span));

        let asm = match self {
            ast::Expr::Path(path) => path.assemble(c, needs)?,
            ast::Expr::While(expr_while) => expr_while.assemble(c, needs)?,
            ast::Expr::For(expr_for) => expr_for.assemble(c, needs)?,
            ast::Expr::Loop(expr_loop) => expr_loop.assemble(c, needs)?,
            ast::Expr::Let(expr_let) => expr_let.assemble(c, needs)?,
            ast::Expr::Group(expr) => expr.expr.assemble(c, needs)?,
            ast::Expr::Unary(expr_unary) => expr_unary.assemble(c, needs)?,
            ast::Expr::Assign(expr_assign) => expr_assign.assemble(c, needs)?,
            ast::Expr::Binary(expr_binary) => expr_binary.assemble(c, needs)?,
            ast::Expr::If(expr_if) => expr_if.assemble(c, needs)?,
            ast::Expr::Index(expr_index_get) => expr_index_get.assemble(c, needs)?,
            ast::Expr::Break(expr_break) => expr_break.assemble(c, needs)?,
            ast::Expr::Continue(expr_continue) => expr_continue.assemble(c, needs)?,
            ast::Expr::Yield(expr_yield) => expr_yield.assemble(c, needs)?,
            ast::Expr::Block(expr_block) => expr_block.assemble(c, needs)?,
            ast::Expr::Return(expr_return) => expr_return.assemble(c, needs)?,
            ast::Expr::Match(expr_match) => expr_match.assemble(c, needs)?,
            ast::Expr::Await(expr_await) => expr_await.assemble(c, needs)?,
            ast::Expr::Try(expr_try) => expr_try.assemble(c, needs)?,
            ast::Expr::Select(expr_select) => expr_select.assemble(c, needs)?,
            ast::Expr::Call(expr_call) => expr_call.assemble(c, needs)?,
            ast::Expr::FieldAccess(expr_field_access) => expr_field_access.assemble(c, needs)?,
            ast::Expr::Closure(expr_closure) => expr_closure.assemble(c, needs)?,
            ast::Expr::Lit(expr_lit) => expr_lit.lit.assemble(c, needs)?,
            ast::Expr::ForceSemi(force_semi) => force_semi.expr.assemble(c, needs)?,
            ast::Expr::Tuple(expr_tuple) => expr_tuple.assemble(c, needs)?,
            ast::Expr::Vec(expr_vec) => expr_vec.assemble(c, needs)?,
            ast::Expr::Object(expr_object) => expr_object.assemble(c, needs)?,
            ast::Expr::Range(expr_range) => expr_range.assemble(c, needs)?,
            ast::Expr::MacroCall(expr_call_macro) => {
                let internal_macro = c.q.builtin_macro_for(&**expr_call_macro)?;

                match &*internal_macro {
                    BuiltInMacro::Template(template) => template.assemble(c, needs)?,
                    BuiltInMacro::Format(format) => format.assemble(c, needs)?,
                    BuiltInMacro::Line(line) => line.value.assemble(c, needs)?,
                    BuiltInMacro::File(file) => file.value.assemble(c, needs)?,
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

                Asm::top(span)
            }
        };

        Ok(asm)
    }
}
