use crate::ir::eval::prelude::*;

impl Eval<&ir::IrBinary> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, ir_binary: &ir::IrBinary, used: Used) -> Result<Self::Output, EvalOutcome> {
        use std::ops::{Add, Mul, Shl, Shr, Sub};

        let span = ir_binary.span();
        self.budget.take(span)?;

        let a = self.eval(&*ir_binary.lhs, used)?;
        let b = self.eval(&*ir_binary.rhs, used)?;

        match (a, b) {
            (IrValue::Integer(a), IrValue::Integer(b)) => match ir_binary.op {
                ir::IrBinaryOp::Add => {
                    return Ok(IrValue::Integer(a.add(&b)));
                }
                ir::IrBinaryOp::Sub => {
                    return Ok(IrValue::Integer(a.sub(&b)));
                }
                ir::IrBinaryOp::Mul => {
                    return Ok(IrValue::Integer(a.mul(&b)));
                }
                ir::IrBinaryOp::Div => {
                    let number = a
                        .checked_div(&b)
                        .ok_or_else(|| IrError::custom(span, "division by zero"))?;
                    return Ok(IrValue::Integer(number));
                }
                ir::IrBinaryOp::Shl => {
                    let b = u32::try_from(b).map_err(|_| {
                        IrError::custom(&ir_binary.rhs, "cannot be converted to shift operand")
                    })?;

                    let n = a.shl(b);
                    return Ok(IrValue::Integer(n));
                }
                ir::IrBinaryOp::Shr => {
                    let b = u32::try_from(b).map_err(|_| {
                        IrError::custom(&ir_binary.rhs, "cannot be converted to shift operand")
                    })?;

                    let n = a.shr(b);
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
