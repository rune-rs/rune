use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::{Compile, Resolve as _};
use crate::CompileError;
use runestick::{Inst, Span};
use std::convert::TryFrom as _;

/// Compile an expr field access, like `<value>.<field>`.
impl Compile<(&ast::ExprFieldAccess, Needs)> for Compiler<'_> {
    fn compile(
        &mut self,
        (expr_field_access, needs): (&ast::ExprFieldAccess, Needs),
    ) -> CompileResult<()> {
        let span = expr_field_access.span();

        // Optimizations!
        //
        // TODO: perform deferred compilation for expressions instead, so we can
        // e.g. inspect if it compiles down to a local access instead of
        // climbing the ast like we do here.
        #[allow(clippy::single_match)]
        match (&*expr_field_access.expr, &expr_field_access.expr_field) {
            (ast::Expr::Path(path), ast::ExprField::LitNumber(n)) => {
                if try_immediate_field_access_optimization(self, span, path, n, needs)? {
                    return Ok(());
                }
            }
            _ => (),
        }

        self.compile((&*expr_field_access.expr, Needs::Value))?;

        // This loop is actually useful.
        #[allow(clippy::never_loop)]
        loop {
            match &expr_field_access.expr_field {
                ast::ExprField::LitNumber(n) => {
                    let index = match n.resolve(&self.storage, &*self.source)? {
                        ast::Number::Integer(n) if n >= 0 => match usize::try_from(n) {
                            Ok(n) => n,
                            Err(..) => break,
                        },
                        _ => break,
                    };

                    self.asm.push(Inst::TupleIndexGet { index }, span);

                    if !needs.value() {
                        self.warnings.not_used(self.source_id, span, self.context());
                        self.asm.push(Inst::Pop, span);
                    }

                    return Ok(());
                }
                ast::ExprField::Ident(ident) => {
                    let field = ident.resolve(&self.storage, &*self.source)?;
                    let slot = self.unit.borrow_mut().new_static_string(field.as_ref())?;

                    self.asm.push(Inst::ObjectSlotIndexGet { slot }, span);

                    if !needs.value() {
                        self.warnings.not_used(self.source_id, span, self.context());
                        self.asm.push(Inst::Pop, span);
                    }

                    return Ok(());
                }
            }
        }

        Err(CompileError::UnsupportedFieldAccess { span })
    }
}

fn try_immediate_field_access_optimization(
    this: &mut Compiler<'_>,
    span: Span,
    path: &ast::Path,
    n: &ast::LitNumber,
    needs: Needs,
) -> CompileResult<bool> {
    let ident = match path.try_as_ident() {
        Some(ident) => ident,
        None => return Ok(false),
    };

    let ident = ident.resolve(this.storage, &*this.source)?;

    let index = match n.resolve(this.storage, &*this.source)? {
        ast::Number::Integer(n) => n,
        _ => return Ok(false),
    };

    let index = match usize::try_from(index) {
        Ok(index) => index,
        Err(..) => return Ok(false),
    };

    let var = match this.scopes.try_get_var(ident.as_ref())? {
        Some(var) => var,
        None => return Ok(false),
    };

    this.asm.push(
        Inst::TupleIndexGetAt {
            offset: var.offset,
            index,
        },
        span,
    );

    if !needs.value() {
        this.warnings.not_used(this.source_id, span, this.context());
        this.asm.push(Inst::Pop, span);
    }

    Ok(true)
}
