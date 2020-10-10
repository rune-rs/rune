use crate::compiling::assemble::prelude::*;

/// Compile a binary expression.
impl Assemble for ast::ExprBinary {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprBinary => {:?}", c.source.source(span));
        log::trace!(
            "ExprBinary {{ lhs => {:?} }}",
            c.source.source(self.lhs.span())
        );
        log::trace!("ExprBinary {{ op => {:?} }}", self.op);
        log::trace!(
            "ExprBinary {{ rhs => {:?} }}",
            c.source.source(self.rhs.span())
        );

        // Special expressions which operates on the stack in special ways.
        if self.op.is_assign() {
            compile_assign_binop(c, &self.lhs, &self.rhs, self.op, needs)?;

            return Ok(());
        }

        if self.op.is_conditional() {
            compile_conditional_binop(c, &self.lhs, &self.rhs, self.op, needs)?;

            return Ok(());
        }

        // NB: need to declare these as anonymous local variables so that they
        // get cleaned up in case there is an early break (return, try, ...).
        self.lhs.assemble(c, Needs::Value)?;
        c.scopes.decl_anon(span)?;

        self.rhs.assemble(c, rhs_needs_of(self.op))?;
        c.scopes.decl_anon(span)?;

        let inst = match self.op {
            ast::BinOp::Eq => Inst::Op { op: InstOp::Eq },
            ast::BinOp::Neq => Inst::Op { op: InstOp::Neq },
            ast::BinOp::Lt => Inst::Op { op: InstOp::Lt },
            ast::BinOp::Gt => Inst::Op { op: InstOp::Gt },
            ast::BinOp::Lte => Inst::Op { op: InstOp::Lte },
            ast::BinOp::Gte => Inst::Op { op: InstOp::Gte },
            ast::BinOp::Is => Inst::Op { op: InstOp::Is },
            ast::BinOp::IsNot => Inst::Op { op: InstOp::IsNot },
            ast::BinOp::And => Inst::Op { op: InstOp::And },
            ast::BinOp::Or => Inst::Op { op: InstOp::Or },
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

        c.asm.push(inst, span);

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        c.scopes.undecl_anon(span, 2)?;
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

fn compile_conditional_binop(
    c: &mut Compiler<'_>,
    lhs: &ast::Expr,
    rhs: &ast::Expr,
    bin_op: ast::BinOp,
    needs: Needs,
) -> CompileResult<()> {
    let span = lhs.span().join(rhs.span());

    let end_label = c.asm.new_label("conditional_end");

    lhs.assemble(c, Needs::Value)?;

    match bin_op {
        ast::BinOp::And => {
            c.asm.jump_if_not_or_pop(end_label, lhs.span());
        }
        ast::BinOp::Or => {
            c.asm.jump_if_or_pop(end_label, lhs.span());
        }
        op => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedBinaryOp { op },
            ));
        }
    }

    rhs.assemble(c, Needs::Value)?;

    c.asm.label(end_label)?;

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(())
}

fn compile_assign_binop(
    c: &mut Compiler<'_>,
    lhs: &ast::Expr,
    rhs: &ast::Expr,
    bin_op: ast::BinOp,
    needs: Needs,
) -> CompileResult<()> {
    let span = lhs.span().join(rhs.span());

    let supported = match lhs {
        // <var> <op> <expr>
        ast::Expr::Path(path) if path.rest.is_empty() => {
            rhs.assemble(c, Needs::Value)?;

            let segment = path
                .first
                .try_as_ident()
                .ok_or_else(|| CompileError::msg(path, "unsupported path segment"))?;
            let ident = segment.resolve(c.storage, &*c.source)?;
            let var = c.scopes.get_var(&*ident, c.source_id, c.visitor, span)?;

            Some(InstTarget::Offset(var.offset))
        }
        // <expr>.<field> <op> <value>
        ast::Expr::FieldAccess(field_access) => {
            field_access.expr.assemble(c, Needs::Value)?;
            rhs.assemble(c, Needs::Value)?;

            // field assignment
            match &field_access.expr_field {
                ast::ExprField::Ident(index) => {
                    let n = index.resolve(c.storage, &*c.source)?;
                    let n = c.unit.new_static_string(index, n.as_ref())?;

                    Some(InstTarget::Field(n))
                }
                ast::ExprField::LitNumber(field) => {
                    let span = field.span();

                    let number = field.resolve(c.storage, &*c.source)?;
                    let index = number.as_tuple_index().ok_or_else(|| {
                        CompileError::new(span, CompileErrorKind::UnsupportedTupleIndex { number })
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
        ast::BinOp::AddAssign => InstAssignOp::Add,
        ast::BinOp::SubAssign => InstAssignOp::Sub,
        ast::BinOp::MulAssign => InstAssignOp::Mul,
        ast::BinOp::DivAssign => InstAssignOp::Div,
        ast::BinOp::RemAssign => InstAssignOp::Rem,
        ast::BinOp::BitAndAssign => InstAssignOp::BitAnd,
        ast::BinOp::BitXorAssign => InstAssignOp::BitXor,
        ast::BinOp::BitOrAssign => InstAssignOp::BitOr,
        ast::BinOp::ShlAssign => InstAssignOp::Shl,
        ast::BinOp::ShrAssign => InstAssignOp::Shr,
        _ => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedBinaryExpr,
            ));
        }
    };

    c.asm.push(Inst::Assign { target, op }, span);

    if needs.value() {
        c.asm.push(Inst::unit(), span);
    }

    Ok(())
}
