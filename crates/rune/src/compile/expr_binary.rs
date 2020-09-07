use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::{Compile, Resolve as _};
use crate::CompileError;
use runestick::Inst;

/// Compile a binary expression.
impl Compile<(&ast::ExprBinary, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_binary, needs): (&ast::ExprBinary, Needs)) -> CompileResult<()> {
        let span = expr_binary.span();
        log::trace!("ExprBinary => {:?}", self.source.source(span));

        // Special expressions which operates on the stack in special ways.
        match expr_binary.op {
            ast::BinOp::Assign
            | ast::BinOp::AddAssign
            | ast::BinOp::SubAssign
            | ast::BinOp::MulAssign
            | ast::BinOp::DivAssign
            | ast::BinOp::RemAssign
            | ast::BinOp::BitAndAssign
            | ast::BinOp::BitXorAssign
            | ast::BinOp::BitOrAssign
            | ast::BinOp::ShlAssign
            | ast::BinOp::ShrAssign => {
                compile_assign_binop(
                    self,
                    &*expr_binary.lhs,
                    &*expr_binary.rhs,
                    expr_binary.op,
                    needs,
                )?;
                return Ok(());
            }
            _ => (),
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
                return Err(CompileError::UnsupportedBinaryOp { span, op });
            }
        }

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        self.scopes.last_mut(span)?.undecl_anon(2, span)?;
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
    compiler: &mut Compiler<'_>,
    lhs: &ast::Expr,
    rhs: &ast::Expr,
    bin_op: ast::BinOp,
    needs: Needs,
) -> CompileResult<()> {
    let span = lhs.span().join(rhs.span());

    // NB: this loop is actually useful in breaking early.
    #[allow(clippy::never_loop)]
    let offset = loop {
        match lhs {
            ast::Expr::ExprFieldAccess(get) => match (&*get.expr, &get.expr_field) {
                (ast::Expr::Path(ast::Path { first, rest }), expr_field) if rest.is_empty() => {
                    let span = first.span();
                    compiler.compile((rhs, Needs::Value))?;
                    let source = compiler.source.clone();
                    let target = first.resolve(compiler.storage, &*source)?;

                    match expr_field {
                        ast::ExprField::Ident(index) => {
                            let span = index.span();
                            let index = index.resolve(compiler.storage, &*compiler.source)?;
                            let index = compiler
                                .unit
                                .borrow_mut()
                                .new_static_string(index.as_ref())?;
                            compiler.asm.push(Inst::String { slot: index }, span);
                        }
                        ast::ExprField::LitNumber(n) => {
                            if compile_tuple_index_set_number(compiler, target.as_ref(), n)? {
                                return Ok(());
                            }
                        }
                    }

                    let var = compiler.scopes.get_var(target.as_ref(), span)?;
                    var.copy(&mut compiler.asm, span, format!("var `{}`", target));

                    compiler.asm.push(Inst::IndexSet, span);
                    return Ok(());
                }
                (ast::Expr::Self_(s), expr_field) => {
                    let span = s.span();
                    compiler.compile((rhs, Needs::Value))?;

                    match expr_field {
                        ast::ExprField::Ident(index) => {
                            let span = index.span();
                            let index = index.resolve(compiler.storage, &*compiler.source)?;
                            let slot = compiler
                                .unit
                                .borrow_mut()
                                .new_static_string(index.as_ref())?;
                            compiler.asm.push(Inst::String { slot }, span);
                        }
                        ast::ExprField::LitNumber(n) => {
                            if compile_tuple_index_set_number(compiler, "self", n)? {
                                return Ok(());
                            }
                        }
                    }

                    let target = compiler.scopes.get_var("self", span)?;
                    target.copy(&mut compiler.asm, span, "self");

                    compiler.asm.push(Inst::IndexSet, span);
                    return Ok(());
                }
                _ => (),
            },
            ast::Expr::Path(ast::Path { first, rest }) if rest.is_empty() => {
                let span = first.span();
                let first = first.resolve(compiler.storage, &*compiler.source)?;
                let var = compiler.scopes.get_var(first.as_ref(), span)?;
                break var.offset;
            }
            _ => (),
        };

        return Err(CompileError::UnsupportedAssignExpr { span });
    };

    compiler.compile((rhs, Needs::Value))?;

    match bin_op {
        ast::BinOp::Assign => {
            compiler.asm.push(Inst::Replace { offset }, span);
        }
        ast::BinOp::AddAssign => {
            compiler.asm.push(Inst::AddAssign { offset }, span);
        }
        ast::BinOp::SubAssign => {
            compiler.asm.push(Inst::SubAssign { offset }, span);
        }
        ast::BinOp::MulAssign => {
            compiler.asm.push(Inst::MulAssign { offset }, span);
        }
        ast::BinOp::DivAssign => {
            compiler.asm.push(Inst::DivAssign { offset }, span);
        }
        ast::BinOp::RemAssign => {
            compiler.asm.push(Inst::RemAssign { offset }, span);
        }
        ast::BinOp::BitAndAssign => {
            compiler.asm.push(Inst::BitAndAssign { offset }, span);
        }
        ast::BinOp::BitXorAssign => {
            compiler.asm.push(Inst::BitXorAssign { offset }, span);
        }
        ast::BinOp::BitOrAssign => {
            compiler.asm.push(Inst::BitOrAssign { offset }, span);
        }
        ast::BinOp::ShlAssign => {
            compiler.asm.push(Inst::ShlAssign { offset }, span);
        }
        ast::BinOp::ShrAssign => {
            compiler.asm.push(Inst::ShrAssign { offset }, span);
        }
        op => {
            return Err(CompileError::UnsupportedAssignBinOp { span, op });
        }
    }

    if needs.value() {
        compiler.asm.push(Inst::Unit, span);
    }

    Ok(())
}

/// Compile a tuple index set operation with a number field.
fn compile_tuple_index_set_number(
    compiler: &mut Compiler<'_>,
    target: &str,
    field: &ast::LitNumber,
) -> CompileResult<bool> {
    let span = field.span();

    let index = match field.resolve(compiler.storage, &*compiler.source)? {
        ast::Number::Integer(n) if n >= 0 => n as usize,
        _ => return Ok(false),
    };

    let var = compiler.scopes.get_var(target, span)?;
    var.copy(&mut compiler.asm, span, format!("var `{}`", target));

    compiler.asm.push(Inst::TupleIndexSet { index }, span);
    Ok(true)
}
