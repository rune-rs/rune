use crate::eval::prelude::*;

impl Eval<&IrBinary> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, ir_binary: &IrBinary, used: Used) -> Result<Self::Output, EvalOutcome> {
        let span = ir_binary.span();
        self.budget.take(span)?;

        let a = self.eval(&*ir_binary.lhs, used)?;
        let b = self.eval(&*ir_binary.rhs, used)?;

        match (a, b) {
            (IrValue::Integer(a), IrValue::Integer(b)) => match ir_binary.op {
                IrBinaryOp::Add => {
                    return Ok(checked_int(
                        a,
                        b,
                        i64::checked_add,
                        "integer overflow",
                        span,
                    )?);
                }
                IrBinaryOp::Sub => {
                    return Ok(checked_int(
                        a,
                        b,
                        i64::checked_sub,
                        "integer underflow",
                        span,
                    )?);
                }
                IrBinaryOp::Mul => {
                    return Ok(checked_int(
                        a,
                        b,
                        i64::checked_mul,
                        "integer overflow",
                        span,
                    )?);
                }
                IrBinaryOp::Div => {
                    return Ok(checked_int(
                        a,
                        b,
                        i64::checked_div,
                        "integer division by zero",
                        span,
                    )?);
                }
                IrBinaryOp::Shl => {
                    let b = u32::try_from(b).map_err(|_| {
                        IrError::custom(&ir_binary.rhs, "cannot be converted to shift operand")
                    })?;

                    let n = a
                        .checked_shl(b)
                        .ok_or_else(|| IrError::custom(span, "integer shift overflow"))?;

                    return Ok(IrValue::Integer(n));
                }
                IrBinaryOp::Shr => {
                    let b = u32::try_from(b).map_err(|_| {
                        IrError::custom(&ir_binary.rhs, "cannot be converted to shift operand")
                    })?;

                    let n = a
                        .checked_shr(b)
                        .ok_or_else(|| IrError::custom(span, "integer shift underflow"))?;

                    return Ok(IrValue::Integer(n));
                }
                IrBinaryOp::Lt => return Ok(IrValue::Bool(a < b)),
                IrBinaryOp::Lte => return Ok(IrValue::Bool(a <= b)),
                IrBinaryOp::Eq => return Ok(IrValue::Bool(a == b)),
                IrBinaryOp::Gt => return Ok(IrValue::Bool(a > b)),
                IrBinaryOp::Gte => return Ok(IrValue::Bool(a >= b)),
            },
            (IrValue::Float(a), IrValue::Float(b)) => {
                match ir_binary.op {
                    IrBinaryOp::Add => return Ok(IrValue::Float(a + b)),
                    IrBinaryOp::Sub => return Ok(IrValue::Float(a - b)),
                    IrBinaryOp::Mul => return Ok(IrValue::Float(a * b)),
                    IrBinaryOp::Div => return Ok(IrValue::Float(a / b)),
                    IrBinaryOp::Lt => return Ok(IrValue::Bool(a < b)),
                    IrBinaryOp::Lte => return Ok(IrValue::Bool(a <= b)),
                    IrBinaryOp::Eq => return Ok(IrValue::Bool(a == b)),
                    IrBinaryOp::Gt => return Ok(IrValue::Bool(a > b)),
                    IrBinaryOp::Gte => return Ok(IrValue::Bool(a >= b)),
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
