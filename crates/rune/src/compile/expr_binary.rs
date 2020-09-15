use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::{Compile, Resolve as _};
use crate::CompileResult;
use crate::{CompileError, CompileErrorKind, Spanned as _};
use runestick::{Inst, InstNumericOp, InstTarget};

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
            ast::BinOp::Add => Inst::StackNumeric {
                op: InstNumericOp::Add,
            },
            ast::BinOp::Sub => Inst::StackNumeric {
                op: InstNumericOp::Sub,
            },
            ast::BinOp::Div => Inst::StackNumeric {
                op: InstNumericOp::Div,
            },
            ast::BinOp::Mul => Inst::StackNumeric {
                op: InstNumericOp::Mul,
            },
            ast::BinOp::Rem => Inst::StackNumeric {
                op: InstNumericOp::Rem,
            },
            ast::BinOp::BitAnd => Inst::StackNumeric {
                op: InstNumericOp::BitAnd,
            },
            ast::BinOp::BitXor => Inst::StackNumeric {
                op: InstNumericOp::BitXor,
            },
            ast::BinOp::BitOr => Inst::StackNumeric {
                op: InstNumericOp::BitOr,
            },
            ast::BinOp::Shl => Inst::StackNumeric {
                op: InstNumericOp::Shl,
            },
            ast::BinOp::Shr => Inst::StackNumeric {
                op: InstNumericOp::Shr,
            },
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

                        this.compile((rhs, Needs::Value))?;
                        this.scopes.decl_anon(rhs.span())?;

                        this.asm.push(Inst::String { slot: index }, span);
                        this.scopes.decl_anon(span)?;

                        this.compile((&*field_access.expr, Needs::Value))?;
                        this.asm.push(Inst::IndexSet, span);
                        this.scopes.undecl_anon(2, span)?;
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
            ast::BinOp::AddAssign => InstNumericOp::Add,
            ast::BinOp::SubAssign => InstNumericOp::Sub,
            ast::BinOp::MulAssign => InstNumericOp::Mul,
            ast::BinOp::DivAssign => InstNumericOp::Div,
            ast::BinOp::RemAssign => InstNumericOp::Rem,
            ast::BinOp::BitAndAssign => InstNumericOp::BitAnd,
            ast::BinOp::BitXorAssign => InstNumericOp::BitXor,
            ast::BinOp::BitOrAssign => InstNumericOp::BitOr,
            ast::BinOp::ShlAssign => InstNumericOp::Shl,
            ast::BinOp::ShrAssign => InstNumericOp::Shr,
            _ => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryExpr,
                ));
            }
        };

        this.asm.push(Inst::AssignNumeric { target, op }, span);
    }

    if needs.value() {
        this.asm.push(Inst::Unit, span);
    }

    Ok(())
}
