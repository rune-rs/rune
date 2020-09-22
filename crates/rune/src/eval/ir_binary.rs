use crate::eval::prelude::*;

impl Eval<&IrBinary> for IrInterpreter<'_> {
    fn eval(&mut self, ir_binary: &IrBinary, used: Used) -> Result<ConstValue, EvalOutcome> {
        let span = ir_binary.span();
        self.budget.take(span)?;

        let a = self.eval(&*ir_binary.lhs, used)?;
        let b = self.eval(&*ir_binary.rhs, used)?;

        match (a, b) {
            (ConstValue::Integer(a), ConstValue::Integer(b)) => match ir_binary.op {
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
                        CompileError::const_error(
                            &ir_binary.rhs,
                            "cannot be converted to shift operand",
                        )
                    })?;

                    let n = a
                        .checked_shl(b)
                        .ok_or_else(|| CompileError::const_error(span, "integer shift overflow"))?;

                    return Ok(ConstValue::Integer(n));
                }
                IrBinaryOp::Shr => {
                    let b = u32::try_from(b).map_err(|_| {
                        CompileError::const_error(
                            &ir_binary.rhs,
                            "cannot be converted to shift operand",
                        )
                    })?;

                    let n = a.checked_shr(b).ok_or_else(|| {
                        CompileError::const_error(span, "integer shift underflow")
                    })?;

                    return Ok(ConstValue::Integer(n));
                }
                IrBinaryOp::Lt => return Ok(ConstValue::Bool(a < b)),
                IrBinaryOp::Lte => return Ok(ConstValue::Bool(a <= b)),
                IrBinaryOp::Eq => return Ok(ConstValue::Bool(a == b)),
                IrBinaryOp::Gt => return Ok(ConstValue::Bool(a > b)),
                IrBinaryOp::Gte => return Ok(ConstValue::Bool(a >= b)),
            },
            (ConstValue::Float(a), ConstValue::Float(b)) => {
                match ir_binary.op {
                    IrBinaryOp::Add => return Ok(ConstValue::Float(a + b)),
                    IrBinaryOp::Sub => return Ok(ConstValue::Float(a - b)),
                    IrBinaryOp::Mul => return Ok(ConstValue::Float(a * b)),
                    IrBinaryOp::Div => return Ok(ConstValue::Float(a / b)),
                    IrBinaryOp::Lt => return Ok(ConstValue::Bool(a < b)),
                    IrBinaryOp::Lte => return Ok(ConstValue::Bool(a <= b)),
                    IrBinaryOp::Eq => return Ok(ConstValue::Bool(a == b)),
                    IrBinaryOp::Gt => return Ok(ConstValue::Bool(a > b)),
                    IrBinaryOp::Gte => return Ok(ConstValue::Bool(a >= b)),
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
) -> Result<ConstValue, CompileError> {
    let n = op(a, b).ok_or_else(|| CompileError::const_error(span, msg))?;
    Ok(ConstValue::Integer(n))
}
