use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::{Compile, Resolve as _};
use crate::CompileResult;
use crate::{CompileError, CompileErrorKind, Spanned as _};
use runestick::Inst;

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

        match expr_binary.op {
            ast::BinOp::Add { .. } => {
                self.asm.push(Inst::Add, span);
            }
            ast::BinOp::Sub { .. } => {
                self.asm.push(Inst::Sub, span);
            }
            ast::BinOp::Div { .. } => {
                self.asm.push(Inst::Div, span);
            }
            ast::BinOp::Mul { .. } => {
                self.asm.push(Inst::Mul, span);
            }
            ast::BinOp::Rem { .. } => {
                self.asm.push(Inst::Rem, span);
            }
            ast::BinOp::Eq { .. } => {
                self.asm.push(Inst::Eq, span);
            }
            ast::BinOp::Neq { .. } => {
                self.asm.push(Inst::Neq, span);
            }
            ast::BinOp::Lt { .. } => {
                self.asm.push(Inst::Lt, span);
            }
            ast::BinOp::Gt { .. } => {
                self.asm.push(Inst::Gt, span);
            }
            ast::BinOp::Lte { .. } => {
                self.asm.push(Inst::Lte, span);
            }
            ast::BinOp::Gte { .. } => {
                self.asm.push(Inst::Gte, span);
            }
            ast::BinOp::Is { .. } => {
                self.asm.push(Inst::Is, span);
            }
            ast::BinOp::IsNot { .. } => {
                self.asm.push(Inst::IsNot, span);
            }
            ast::BinOp::And { .. } => {
                self.asm.push(Inst::And, span);
            }
            ast::BinOp::Or { .. } => {
                self.asm.push(Inst::Or, span);
            }
            ast::BinOp::BitAnd { .. } => {
                self.asm.push(Inst::BitAnd, span);
            }
            ast::BinOp::BitXor { .. } => {
                self.asm.push(Inst::BitXor, span);
            }
            ast::BinOp::BitOr { .. } => {
                self.asm.push(Inst::BitOr, span);
            }
            ast::BinOp::Shl { .. } => {
                self.asm.push(Inst::Shl, span);
            }
            ast::BinOp::Shr { .. } => {
                self.asm.push(Inst::Shr, span);
            }
            op => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryOp { op },
                ));
            }
        }

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
                let ident = path.first.resolve(this.storage, &*this.source)?;
                let var = this
                    .scopes
                    .get_var(&*ident, this.source.url(), this.visitor, span)?;
                Some(var.offset)
            }
            // Note: we would like to support assign operators for tuples and
            // objects as well, but these would require a different addressing
            // mode for the operations which would require adding instructions
            // or more capabilities to existing ones.

            // See
            _ => None,
        };

        let offset = match supported {
            Some(offset) => offset,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryExpr,
                ));
            }
        };

        this.compile((rhs, Needs::Value))?;

        match bin_op {
            ast::BinOp::AddAssign => {
                this.asm.push(Inst::AddAssign { offset }, span);
            }
            ast::BinOp::SubAssign => {
                this.asm.push(Inst::SubAssign { offset }, span);
            }
            ast::BinOp::MulAssign => {
                this.asm.push(Inst::MulAssign { offset }, span);
            }
            ast::BinOp::DivAssign => {
                this.asm.push(Inst::DivAssign { offset }, span);
            }
            ast::BinOp::RemAssign => {
                this.asm.push(Inst::RemAssign { offset }, span);
            }
            ast::BinOp::BitAndAssign => {
                this.asm.push(Inst::BitAndAssign { offset }, span);
            }
            ast::BinOp::BitXorAssign => {
                this.asm.push(Inst::BitXorAssign { offset }, span);
            }
            ast::BinOp::BitOrAssign => {
                this.asm.push(Inst::BitOrAssign { offset }, span);
            }
            ast::BinOp::ShlAssign => {
                this.asm.push(Inst::ShlAssign { offset }, span);
            }
            ast::BinOp::ShrAssign => {
                this.asm.push(Inst::ShrAssign { offset }, span);
            }
            _ => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryExpr,
                ));
            }
        }
    }

    if needs.value() {
        this.asm.push(Inst::Unit, span);
    }

    Ok(())
}
