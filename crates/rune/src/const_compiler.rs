use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::query::Query;
use crate::{CompileError, CompileErrorKind, Resolve as _, Spanned as _};
use runestick::{CompileMetaKind, ConstValue, Item, Source, Span};
use std::convert::TryFrom as _;

/// State for constants processing.
#[derive(Default)]
pub(crate) struct Consts {
    /// Const expression that have been resolved.
    pub(crate) resolved: HashMap<Item, ConstValue>,
    /// Constant expressions being processed.
    pub(crate) processing: HashSet<Item>,
}

/// The compiler phase which evaluates constants.
pub(crate) struct ConstCompiler<'a> {
    /// The item where the constant expression is located.
    pub(crate) item: Item,
    /// Source file used in processing.
    pub(crate) source: &'a Source,
    /// Query engine to look for constant expressions.
    pub(crate) query: &'a mut Query,
}

impl ConstCompiler<'_> {
    pub(crate) fn eval_expr(
        &mut self,
        expr: &ast::Expr,
        unused: bool,
    ) -> Result<ConstValue, CompileError> {
        log::trace!("processing constant: {}", self.item);

        if let Some(const_value) = self.query.consts.borrow().resolved.get(&self.item).cloned() {
            return Ok(const_value);
        }

        if !self
            .query
            .consts
            .borrow_mut()
            .processing
            .insert(self.item.clone())
        {
            return Err(CompileError::new(expr, CompileErrorKind::ConstCycle));
        }

        let const_value = self.eval_inner_expr(expr, unused)?;

        if self
            .query
            .consts
            .borrow_mut()
            .resolved
            .insert(self.item.clone(), const_value.clone())
            .is_some()
        {
            return Err(CompileError::new(expr, CompileErrorKind::ConstCycle));
        }

        Ok(const_value)
    }

    /// Eval the interior expression.
    fn eval_inner_expr(
        &mut self,
        expr: &ast::Expr,
        unused: bool,
    ) -> Result<ConstValue, CompileError> {
        let const_value = loop {
            match expr {
                ast::Expr::ExprBinary(binary) => {
                    let lhs = self.eval_inner_expr(&binary.lhs, unused)?;
                    let rhs = self.eval_inner_expr(&binary.rhs, unused)?;

                    let span = binary.lhs.span().join(binary.rhs.span());

                    match (lhs, rhs) {
                        (ConstValue::Integer(a), ConstValue::Integer(b)) => {
                            match binary.op {
                                ast::BinOp::Add => {
                                    break checked_int(
                                        a,
                                        b,
                                        i64::checked_add,
                                        "integer overflow",
                                        span,
                                    )?
                                }
                                ast::BinOp::Sub => {
                                    break checked_int(
                                        a,
                                        b,
                                        i64::checked_sub,
                                        "integer underflow",
                                        span,
                                    )?
                                }
                                ast::BinOp::Mul => {
                                    break checked_int(
                                        a,
                                        b,
                                        i64::checked_mul,
                                        "integer overflow",
                                        span,
                                    )?
                                }
                                ast::BinOp::Div => {
                                    break checked_int(
                                        a,
                                        b,
                                        i64::checked_div,
                                        "integer division by zero",
                                        span,
                                    )?
                                }
                                ast::BinOp::Shl => {
                                    let b = u32::try_from(b).map_err(|_| {
                                        CompileError::const_error(
                                            &binary.rhs,
                                            "cannot be converted to shift operand",
                                        )
                                    })?;

                                    let n = a.checked_shl(b).ok_or_else(|| {
                                        CompileError::const_error(span, "integer shift overflow")
                                    })?;

                                    break ConstValue::Integer(n);
                                }
                                ast::BinOp::Shr => {
                                    let b = u32::try_from(b).map_err(|_| {
                                        CompileError::const_error(
                                            &binary.rhs,
                                            "cannot be converted to shift operand",
                                        )
                                    })?;

                                    let n = a.checked_shr(b).ok_or_else(|| {
                                        CompileError::const_error(span, "integer shift underflow")
                                    })?;

                                    break ConstValue::Integer(n);
                                }
                                ast::BinOp::Lt => break ConstValue::Bool(a < b),
                                ast::BinOp::Lte => break ConstValue::Bool(a <= b),
                                ast::BinOp::Eq => break ConstValue::Bool(a == b),
                                ast::BinOp::Gt => break ConstValue::Bool(a > b),
                                ast::BinOp::Gte => break ConstValue::Bool(a >= b),
                                _ => (),
                            };
                        }
                        _ => (),
                    }
                }
                ast::Expr::ExprLit(expr_lit) => match &expr_lit.lit {
                    ast::Lit::Bool(b) => {
                        break ConstValue::Bool(b.value);
                    }
                    ast::Lit::Number(n) => {
                        let n = n.resolve(&self.query.storage, self.source)?;

                        match n {
                            ast::Number::Integer(n) => break ConstValue::Integer(n),
                            _ => (),
                        }
                    }
                    ast::Lit::Str(s) => {
                        let s = s.resolve(&self.query.storage, self.source)?;
                        let slot = self.query.unit.borrow_mut().new_static_string(s.as_ref())?;
                        break ConstValue::String(slot);
                    }
                    _ => (),
                },
                ast::Expr::Path(path) => {
                    if let Some(ident) = path.try_as_ident() {
                        let ident = ident.resolve(&self.query.storage, self.source)?;
                        let const_value = self.resolve_var(ident.as_ref(), path.span(), unused)?;
                        break const_value;
                    }
                }
                _ => (),
            }

            return Err(CompileError::new(expr, CompileErrorKind::NotConst));
        };

        Ok(const_value)
    }

    /// Resolve the given constant.
    fn resolve_var(
        &mut self,
        ident: &str,
        span: Span,
        unused: bool,
    ) -> Result<ConstValue, CompileError> {
        let mut base = self.item.clone();

        while !base.is_empty() {
            base.pop();
            let item = base.extended(ident);

            if let Some(const_value) = self.query.consts.borrow().resolved.get(&item).cloned() {
                return Ok(const_value);
            }

            let meta = match self.query.query_meta_with_use(&item, unused)? {
                Some(meta) => meta,
                None => continue,
            };

            match &meta.kind {
                CompileMetaKind::Const { const_value, .. } => return Ok(const_value.clone()),
                _ => {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedMetaConst { meta },
                    ));
                }
            }
        }

        Err(CompileError::new(span, CompileErrorKind::NotConst))
    }
}

fn checked_int(
    a: i64,
    b: i64,
    op: impl FnOnce(i64, i64) -> Option<i64>,
    msg: &'static str,
    span: Span,
) -> Result<ConstValue, CompileError> {
    let n = op(a, b).ok_or_else(|| CompileError::const_error(span, msg))?;
    Ok(ConstValue::Integer(n))
}
