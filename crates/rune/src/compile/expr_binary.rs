use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::{Compile, Resolve as _};
use crate::CompileResult;
use crate::{CompileError, CompileErrorKind, Spanned as _};
use runestick::{Inst, InstOp, InstTarget};

/// Compile a binary expression.
impl Compile<(&ast::ExprBinary, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_binary, needs): (&ast::ExprBinary, Needs)) -> CompileResult<()> {
        let span = expr_binary.span();
        log::trace!("ExprBinary => {:?}", self.source.source(span));

        // Special expressions which operates on the stack in special ways.
        if expr_binary.op.is_assign() {
            compile_assign_binop(
                self,
                &*expr_binary.lhs,
                &*expr_binary.rhs,
                expr_binary.op,
                needs,
            )?;

            return Ok(());
        }

        // NB: need to declare these as anonymous local variables so that they
        // get cleaned up in case there is an early break (return, try, ...).
        self.compile((&*expr_binary.lhs, Needs::Value))?;
        self.scopes.decl_anon(span)?;

        self.compile((&*expr_binary.rhs, rhs_needs_of(expr_binary.op)))?;
        self.scopes.decl_anon(span)?;

        let inst = match expr_binary.op {
            ast::BinOp::Eq => Inst::Eq,
            ast::BinOp::Neq => Inst::Neq,
            ast::BinOp::Lt => Inst::Lt,
            ast::BinOp::Gt => Inst::Gt,
            ast::BinOp::Lte => Inst::Lte,
            ast::BinOp::Gte => Inst::Gte,
            ast::BinOp::Is => Inst::Is,
            ast::BinOp::IsNot => Inst::IsNot,
            ast::BinOp::And => Inst::And,
            ast::BinOp::Or => Inst::Or,
            ast::BinOp::Add => Inst::Op { op: InstOp::Add },
            ast::BinOp::Sub => Inst::Op { op: InstOp::Sub },
            ast::BinOp::Div => Inst::Op { op: InstOp::Div },
            ast::BinOp::Mul => Inst::Op { op: InstOp::Mul },
            ast::BinOp::Rem => Inst::Op { op: InstOp::Rem },
            ast::BinOp::BitAnd => Inst::Op { op: InstOp::BitAnd },
            ast::BinOp::BitXor => Inst::Op { op: InstOp::BitXor },
            ast::BinOp::BitOr => Inst::Op { op: InstOp::BitOr },
            ast::BinOp::Shl => Inst::Op { op: InstOp::Shl },
            ast::BinOp::Shr => Inst::Op { op: InstOp::Shr },
            op => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryOp { op },
                ));
            }
        };

        self.asm.push(inst, span);

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        self.scopes.undecl_anon(2, span)?;
        Ok(())
    }
}

/// Get the need of the right-hand side operator from the type of the
/// operator.
fn rhs_needs_of(op: ast::BinOp) -> Needs {
    match op {
        ast::BinOp::Is | ast::BinOp::IsNot => Needs::Type,
        _ => Needs::Value,
    }
}

fn compile_assign_binop(
    this: &mut Compiler<'_>,
    lhs: &ast::Expr,
    rhs: &ast::Expr,
    bin_op: ast::BinOp,
    needs: Needs,
) -> CompileResult<()> {
    let span = lhs.span().join(rhs.span());

    // assignments
    if let ast::BinOp::Assign = bin_op {
        let supported = match lhs {
            // <var> = <value>
            ast::Expr::Path(path) if path.rest.is_empty() => {
                this.compile((rhs, Needs::Value))?;

                let ident = path.first.resolve(this.storage, &*this.source)?;
                let var = this
                    .scopes
                    .get_var(&*ident, this.source.url(), this.visitor, span)?;
                this.asm.push(Inst::Replace { offset: var.offset }, span);

                true
            }
            // <expr>.<field> = <value>
            ast::Expr::ExprFieldAccess(field_access) => {
                // field assignment
                match &field_access.expr_field {
                    ast::ExprField::Ident(index) => {
                        let span = index.span();

                        let index = index.resolve(this.storage, &*this.source)?;
                        let index = this.unit.borrow_mut().new_static_string(index.as_ref())?;

                        this.compile((&*field_access.expr, Needs::Value))?;
                        this.scopes.decl_anon(span)?;

                        this.asm.push(Inst::String { slot: index }, span);
                        this.scopes.decl_anon(span)?;

                        this.compile((rhs, Needs::Value))?;
                        this.scopes.decl_anon(rhs.span())?;

                        this.asm.push(Inst::IndexSet, span);
                        this.scopes.undecl_anon(3, span)?;
                        true
                    }
                    ast::ExprField::LitNumber(field) => {
                        let span = field.span();
                        let number = field.resolve(this.storage, &*this.source)?;
                        let index = number.into_tuple_index().ok_or_else(|| {
                            CompileError::new(
                                span,
                                CompileErrorKind::UnsupportedTupleIndex { number },
                            )
                        })?;

                        this.compile((rhs, Needs::Value))?;
                        this.scopes.decl_anon(rhs.span())?;

                        this.compile((&*field_access.expr, Needs::Value))?;
                        this.asm.push(Inst::TupleIndexSet { index }, span);
                        this.scopes.undecl_anon(1, span)?;
                        true
                    }
                }
            }
            _ => false,
        };

        if !supported {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedAssignExpr,
            ));
        }
    } else {
        let supported = match lhs {
            // <var> <op> <expr>
            ast::Expr::Path(path) if path.rest.is_empty() => {
                this.compile((rhs, Needs::Value))?;

                let ident = path.first.resolve(this.storage, &*this.source)?;
                let var = this
                    .scopes
                    .get_var(&*ident, this.source.url(), this.visitor, span)?;

                Some(InstTarget::Offset(var.offset))
            }
            // <expr>.<field> <op> <value>
            ast::Expr::ExprFieldAccess(field_access) => {
                this.compile((&*field_access.expr, Needs::Value))?;
                this.compile((rhs, Needs::Value))?;

                // field assignment
                match &field_access.expr_field {
                    ast::ExprField::Ident(index) => {
                        let index = index.resolve(this.storage, &*this.source)?;
                        let index = this.unit.borrow_mut().new_static_string(index.as_ref())?;

                        Some(InstTarget::Field(index))
                    }
                    ast::ExprField::LitNumber(field) => {
                        let span = field.span();

                        let number = field.resolve(this.storage, &*this.source)?;
                        let index = number.into_tuple_index().ok_or_else(|| {
                            CompileError::new(
                                span,
                                CompileErrorKind::UnsupportedTupleIndex { number },
                            )
                        })?;

                        Some(InstTarget::TupleField(index))
                    }
                }
            }
            _ => None,
        };

        let target = match supported {
            Some(target) => target,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryExpr,
                ));
            }
        };

        let op = match bin_op {
            ast::BinOp::AddAssign => InstOp::Add,
            ast::BinOp::SubAssign => InstOp::Sub,
            ast::BinOp::MulAssign => InstOp::Mul,
            ast::BinOp::DivAssign => InstOp::Div,
            ast::BinOp::RemAssign => InstOp::Rem,
            ast::BinOp::BitAndAssign => InstOp::BitAnd,
            ast::BinOp::BitXorAssign => InstOp::BitXor,
            ast::BinOp::BitOrAssign => InstOp::BitOr,
            ast::BinOp::ShlAssign => InstOp::Shl,
            ast::BinOp::ShrAssign => InstOp::Shr,
            _ => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryExpr,
                ));
            }
        };

        this.asm.push(Inst::Assign { target, op }, span);
    }

    if needs.value() {
        this.asm.push(Inst::unit(), span);
    }

    Ok(())
}
