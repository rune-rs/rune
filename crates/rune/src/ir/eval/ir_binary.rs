use crate::ir::eval::prelude::*;

impl IrEval for ir::IrBinary {
    type Output = IrValue;

    fn eval(
        &self,
        interp: &mut IrInterpreter<'_>,
        used: Used,
    ) -> Result<Self::Output, IrEvalOutcome> {
        use std::ops::{Add, Mul, Shl, Shr, Sub};

        let span = self.span();
        interp.budget.take(span)?;

        let a = self.lhs.eval(interp, used)?;
        let b = self.rhs.eval(interp, used)?;

        match (a, b) {
            (IrValue::Integer(a), IrValue::Integer(b)) => match self.op {
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
                        IrError::custom(&self.rhs, "cannot be converted to shift operand")
                    })?;

                    let n = a.shl(b);
                    return Ok(IrValue::Integer(n));
                }
                ir::IrBinaryOp::Shr => {
                    let b = u32::try_from(b).map_err(|_| {
                        IrError::custom(&self.rhs, "cannot be converted to shift operand")
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
                match self.op {
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
            (IrValue::String(a), IrValue::String(b)) => match self.op {
                ir::IrBinaryOp::Add => {
                    return Ok(IrValue::String(add_strings(span, &a, &b)?));
                }
                _ => (),
            },
            _ => (),
        }

        Err(IrEvalOutcome::not_const(span))
    }
}

fn add_strings(
    span: Span,
    a: &Shared<String>,
    b: &Shared<String>,
) -> Result<Shared<String>, IrError> {
    let a = a.borrow_ref().map_err(|e| IrError::new(span, e))?;
    let b = b.borrow_ref().map_err(|e| IrError::new(span, e))?;

    let mut a = String::from(&*a);
    a.push_str(&b);
    Ok(Shared::new(a))
}
