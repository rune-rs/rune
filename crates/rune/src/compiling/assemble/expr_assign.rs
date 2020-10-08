use crate::compiling::assemble::prelude::*;

/// Compile an `.await` expression.
impl Assemble for ast::ExprAssign {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprAssign => {:?}", c.source.source(span));

        let supported = match &self.lhs {
            // <var> = <value>
            ast::Expr::Path(path) if path.rest.is_empty() => {
                self.rhs.assemble(c, Needs::Value)?;

                let segment = path
                    .first
                    .try_as_ident()
                    .ok_or_else(|| CompileError::internal_unsupported_path(path))?;
                let ident = segment.resolve(c.storage, &*c.source)?;
                let var = c.scopes.get_var(&*ident, c.source_id, c.visitor, span)?;
                c.asm.push(Inst::Replace { offset: var.offset }, span);
                true
            }
            // <expr>.<field> = <value>
            ast::Expr::ExprFieldAccess(field_access) => {
                // field assignment
                match &field_access.expr_field {
                    ast::ExprField::Ident(index) => {
                        let span = index.span();

                        let slot = index.resolve(c.storage, &*c.source)?;
                        let slot = c.unit.new_static_string(index, slot.as_ref())?;

                        self.rhs.assemble(c, Needs::Value)?;
                        c.scopes.decl_anon(self.rhs.span())?;

                        field_access.expr.assemble(c, Needs::Value)?;
                        c.scopes.decl_anon(span)?;

                        c.asm.push(Inst::String { slot }, span);
                        c.scopes.decl_anon(span)?;

                        c.asm.push(Inst::IndexSet, span);
                        c.scopes.undecl_anon(span, 3)?;
                        true
                    }
                    ast::ExprField::LitNumber(field) => {
                        let span = field.span();
                        let number = field.resolve(c.storage, &*c.source)?;
                        let index = number.as_tuple_index().ok_or_else(|| {
                            CompileError::new(
                                span,
                                CompileErrorKind::UnsupportedTupleIndex { number },
                            )
                        })?;

                        self.rhs.assemble(c, Needs::Value)?;
                        c.scopes.decl_anon(self.rhs.span())?;

                        field_access.expr.assemble(c, Needs::Value)?;
                        c.asm.push(Inst::TupleIndexSet { index }, span);
                        c.scopes.undecl_anon(span, 1)?;
                        true
                    }
                }
            }
            ast::Expr::ExprIndex(expr_index_get) => {
                let span = expr_index_get.span();
                log::trace!("ExprIndexSet => {:?}", c.source.source(span));

                self.rhs.assemble(c, Needs::Value)?;
                c.scopes.decl_anon(span)?;

                expr_index_get.target.assemble(c, Needs::Value)?;
                c.scopes.decl_anon(span)?;

                expr_index_get.index.assemble(c, Needs::Value)?;
                c.scopes.decl_anon(span)?;

                c.asm.push(Inst::IndexSet, span);
                c.scopes.undecl_anon(span, 3)?;
                true
            }
            _ => false,
        };

        if !supported {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedAssignExpr,
            ));
        }

        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }

        Ok(())
    }
}
