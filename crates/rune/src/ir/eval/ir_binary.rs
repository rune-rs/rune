use crate::ir::eval::prelude::*;

impl Eval<&ir::IrBinary> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, ir_binary: &ir::IrBinary, used: Used) -> Result<Self::Output, EvalOutcome> {
        let span = ir_binary.span();
        self.budget.take(span)?;

        let a = self.eval(&*ir_binary.lhs, used)?;
        let b = self.eval(&*ir_binary.rhs, used)?;

        match (a, b) {
            (IrValue::Integer(a), IrValue::Integer(b)) => match ir_binary.op {
                ir::IrBinaryOp::Add => {
                    return Ok(checked_int(
                        a,
                        b,
                        i64::checked_add,
                        "integer overflow",
                        span,
                    )?);
                }
                ir::IrBinaryOp::Sub => {
                    return Ok(checked_int(
                        a,
                        b,
                        i64::checked_sub,
                        "integer underflow",
                        span,
                    )?);
                }
                ir::IrBinaryOp::Mul => {
                    return Ok(checked_int(
                        a,
                        b,
                        i64::checked_mul,
                        "integer overflow",
                        span,
                    )?);
                }
                ir::IrBinaryOp::Div => {
                    return Ok(checked_int(
                        a,
                        b,
                        i64::checked_div,
                        "integer division by zero",
                        span,
                    )?);
                }
                ir::IrBinaryOp::Shl => {
                    let b = u32::try_from(b).map_err(|_| {
                        IrError::custom(&ir_binary.rhs, "cannot be converted to shift operand")
                    })?;

                    let n = a
                        .checked_shl(b)
                        .ok_or_else(|| IrError::custom(span, "integer shift overflow"))?;

                    return Ok(IrValue::Integer(n));
                }
                ir::IrBinaryOp::Shr => {
                    let b = u32::try_from(b).map_err(|_| {
                        IrError::custom(&ir_binary.rhs, "cannot be converted to shift operand")
                    })?;

                    let n = a
                        .checked_shr(b)
                        .ok_or_else(|| IrError::custom(span, "integer shift underflow"))?;

                    return Ok(IrValue::Integer(n));
                }
                ir::IrBinaryOp::Lt => return Ok(IrValue::Bool(a < b)),
                ir::IrBinaryOp::Lte => return Ok(IrValue::Bool(a <= b)),
                ir::IrBinaryOp::Eq => return Ok(IrValue::Bool(a == b)),
                ir::IrBinaryOp::Gt => return Ok(IrValue::Bool(a > b)),
                ir::IrBinaryOp::Gte => return Ok(IrValue::Bool(a >= b)),
            },
            (IrValue::Float(a), IrValue::Float(b)) => {
                match ir_binary.op {
                    ir::IrBinaryOp::Add => return Ok(IrValue::Float(a + b)),
                    ir::IrBinaryOp::Sub => return Ok(IrValue::Float(a - b)),
                    ir::IrBinaryOp::Mul => return Ok(IrValue::Float(a * b)),
                    ir::IrBinaryOp::Div => return Ok(IrValue::Float(a / b)),
                    ir::IrBinaryOp::Lt => return Ok(IrValue::Bool(a < b)),
                    ir::IrBinaryOp::Lte => return Ok(IrValue::Bool(a <= b)),
                    ir::IrBinaryOp::Eq => return Ok(IrValue::Bool(a == b)),
                    ir::IrBinaryOp::Gt => return Ok(IrValue::Bool(a > b)),
                    ir::IrBinaryOp::Gte => return Ok(IrValue::Bool(a >= b)),
                    _ => (),
                };
            }
            _ => (),
        }

        Err(EvalOutcome::not_const(span))
    }
}

fn checked_int(
    a: i64,
    b: i64,
    op: impl FnOnce(i64, i64) -> Option<i64>,
    msg: &'static str,
    span: Span,
) -> Result<IrValue, IrError> {
    let n = op(a, b).ok_or_else(|| IrError::custom(span, msg))?;
    Ok(IrValue::Integer(n))
}
