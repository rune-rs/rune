use crate::ast;
use crate::{CompileError, CompileErrorKind, Resolve as _, Storage, UnitBuilder};
use runestick::{ConstValue, Source};
use std::cell::RefCell;
use std::rc::Rc;

/// The compiler phase which evaluates constants.
pub(crate) struct ConstCompiler<'a> {
    pub(crate) storage: &'a Storage,
    pub(crate) source: &'a Source,
    pub(crate) unit: &'a Rc<RefCell<UnitBuilder>>,
}

impl ConstCompiler<'_> {
    pub(crate) fn eval_expr(&mut self, expr: &ast::Expr) -> Result<ConstValue, CompileError> {
        match expr {
            ast::Expr::ExprLit(expr_lit) => match &expr_lit.lit {
                ast::Lit::Bool(b) => {
                    return Ok(ConstValue::Bool(b.value));
                }
                ast::Lit::Str(s) => {
                    let s = s.resolve(self.storage, self.source)?;
                    let slot = self.unit.borrow_mut().new_static_string(s.as_ref())?;
                    return Ok(ConstValue::String(slot));
                }
                _ => (),
            },
            _ => (),
        }

        Err(CompileError::new(expr, CompileErrorKind::NotConst))
    }
}
