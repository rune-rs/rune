use crate::compiling::compile::prelude::*;

/// Compile an `.await` expression.
impl Compile<(&ast::ExprAssign, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_assign, needs): (&ast::ExprAssign, Needs)) -> CompileResult<()> {
        let span = expr_assign.span();
        log::trace!("ExprAssign => {:?}", self.source.source(span));

        let supported = match &expr_assign.lhs {
            // <var> = <value>
            ast::Expr::Path(path) if path.rest.is_empty() => {
                self.compile((&expr_assign.rhs, Needs::Value))?;

                let segment = path
                    .first
                    .try_as_ident()
                    .ok_or_else(|| CompileError::internal_unsupported_path(path))?;
                let ident = segment.resolve(self.storage, &*self.source)?;
                let var = self
                    .scopes
                    .get_var(&*ident, self.source_id, self.visitor, span)?;
                self.asm.push(Inst::Replace { offset: var.offset }, span);
                true
            }
            // <expr>.<field> = <value>
            ast::Expr::ExprFieldAccess(field_access) => {
                // field assignment
                match &field_access.expr_field {
                    ast::ExprField::Ident(index) => {
                        let span = index.span();

                        let slot = index.resolve(self.storage, &*self.source)?;
                        let slot = self.unit.new_static_string(index, slot.as_ref())?;

                        self.compile((&expr_assign.rhs, Needs::Value))?;
                        self.scopes.decl_anon(expr_assign.rhs.span())?;

                        self.compile((&field_access.expr, Needs::Value))?;
                        self.scopes.decl_anon(span)?;

                        self.asm.push(Inst::String { slot }, span);
                        self.scopes.decl_anon(span)?;

                        self.asm.push(Inst::IndexSet, span);
                        self.scopes.undecl_anon(span, 3)?;
                        true
                    }
                    ast::ExprField::LitNumber(field) => {
                        let span = field.span();
                        let number = field.resolve(self.storage, &*self.source)?;
                        let index = number.as_tuple_index().ok_or_else(|| {
                            CompileError::new(
                                span,
                                CompileErrorKind::UnsupportedTupleIndex { number },
                            )
                        })?;

                        self.compile((&expr_assign.rhs, Needs::Value))?;
                        self.scopes.decl_anon(expr_assign.rhs.span())?;

                        self.compile((&field_access.expr, Needs::Value))?;
                        self.asm.push(Inst::TupleIndexSet { index }, span);
                        self.scopes.undecl_anon(span, 1)?;
                        true
                    }
                }
            }
            ast::Expr::ExprIndex(expr_index_get) => {
                let span = expr_index_get.span();
                log::trace!("ExprIndexSet => {:?}", self.source.source(span));

                self.compile((&expr_assign.rhs, Needs::Value))?;
                self.scopes.decl_anon(span)?;

                self.compile((&expr_index_get.target, Needs::Value))?;
                self.scopes.decl_anon(span)?;

                self.compile((&expr_index_get.index, Needs::Value))?;
                self.scopes.decl_anon(span)?;

                self.asm.push(Inst::IndexSet, span);
                self.scopes.undecl_anon(span, 3)?;
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
            self.asm.push(Inst::unit(), span);
        }

        Ok(())
    }
}
