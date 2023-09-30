use core::ops::{Add, Mul, Shl, Shr, Sub};

use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::{Box, HashMap, String, Vec};
use crate::ast::{Span, Spanned};
use crate::compile::ir;
use crate::compile::{self, WithSpan};
use crate::query::Used;
use crate::runtime::Shared;

/// Process an ir value as a boolean.
fn as_bool(span: Span, value: ir::Value) -> compile::Result<bool> {
    value
        .into_bool()
        .map_err(|actual| compile::Error::expected_type::<_, bool>(span, &actual))
}

/// The outcome of a constant evaluation.
pub enum EvalOutcome {
    /// Encountered expression that is not a valid constant expression.
    NotConst(Span),
    /// A compile error.
    Error(compile::Error),
    /// Break until the next loop, or the optional label.
    Break(Span, Option<Box<str>>, Option<ir::Value>),
}

impl EvalOutcome {
    /// Encountered ast that is not a constant expression.
    pub(crate) fn not_const<S>(spanned: S) -> Self
    where
        S: Spanned,
    {
        Self::NotConst(spanned.span())
    }
}

impl<T> From<T> for EvalOutcome
where
    compile::Error: From<T>,
{
    fn from(error: T) -> Self {
        Self::Error(compile::Error::from(error))
    }
}

fn eval_ir_assign(
    ir: &ir::IrAssign,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    interp.budget.take(ir)?;
    let value = eval_ir(&ir.value, interp, used)?;

    interp
        .scopes
        .mut_target(&ir.target, move |t| ir.op.assign(ir, t, value))?;

    Ok(ir::Value::EmptyTuple)
}

fn eval_ir_binary(
    ir: &ir::IrBinary,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    let span = ir.span();
    interp.budget.take(span)?;

    let a = eval_ir(&ir.lhs, interp, used)?;
    let b = eval_ir(&ir.rhs, interp, used)?;

    match (a, b) {
        (ir::Value::Integer(a), ir::Value::Integer(b)) => match ir.op {
            ir::IrBinaryOp::Add => {
                return Ok(ir::Value::Integer(a.add(&b)));
            }
            ir::IrBinaryOp::Sub => {
                return Ok(ir::Value::Integer(a.sub(&b)));
            }
            ir::IrBinaryOp::Mul => {
                return Ok(ir::Value::Integer(a.mul(&b)));
            }
            ir::IrBinaryOp::Div => {
                let number = a
                    .checked_div(b)
                    .ok_or_else(|| compile::Error::msg(span, "division by zero"))?;
                return Ok(ir::Value::Integer(number));
            }
            ir::IrBinaryOp::Shl => {
                let b = u32::try_from(b).map_err(|_| {
                    compile::Error::msg(&ir.rhs, "cannot be converted to shift operand")
                })?;

                let n = a.shl(b);
                return Ok(ir::Value::Integer(n));
            }
            ir::IrBinaryOp::Shr => {
                let b = u32::try_from(b).map_err(|_| {
                    compile::Error::msg(&ir.rhs, "cannot be converted to shift operand")
                })?;

                let n = a.shr(b);
                return Ok(ir::Value::Integer(n));
            }
            ir::IrBinaryOp::Lt => return Ok(ir::Value::Bool(a < b)),
            ir::IrBinaryOp::Lte => return Ok(ir::Value::Bool(a <= b)),
            ir::IrBinaryOp::Eq => return Ok(ir::Value::Bool(a == b)),
            ir::IrBinaryOp::Gt => return Ok(ir::Value::Bool(a > b)),
            ir::IrBinaryOp::Gte => return Ok(ir::Value::Bool(a >= b)),
        },
        (ir::Value::Float(a), ir::Value::Float(b)) => {
            #[allow(clippy::float_cmp)]
            match ir.op {
                ir::IrBinaryOp::Add => return Ok(ir::Value::Float(a + b)),
                ir::IrBinaryOp::Sub => return Ok(ir::Value::Float(a - b)),
                ir::IrBinaryOp::Mul => return Ok(ir::Value::Float(a * b)),
                ir::IrBinaryOp::Div => return Ok(ir::Value::Float(a / b)),
                ir::IrBinaryOp::Lt => return Ok(ir::Value::Bool(a < b)),
                ir::IrBinaryOp::Lte => return Ok(ir::Value::Bool(a <= b)),
                ir::IrBinaryOp::Eq => return Ok(ir::Value::Bool(a == b)),
                ir::IrBinaryOp::Gt => return Ok(ir::Value::Bool(a > b)),
                ir::IrBinaryOp::Gte => return Ok(ir::Value::Bool(a >= b)),
                _ => (),
            };
        }
        (ir::Value::String(a), ir::Value::String(b)) => {
            if let ir::IrBinaryOp::Add = ir.op {
                return Ok(ir::Value::String(add_strings(span, &a, &b)?));
            }
        }
        _ => (),
    }

    return Err(EvalOutcome::not_const(span));

    fn add_strings(
        span: Span,
        a: &Shared<String>,
        b: &Shared<String>,
    ) -> compile::Result<Shared<String>> {
        let a = a.borrow_ref().with_span(span)?;
        let b = b.borrow_ref().with_span(span)?;
        let mut a = (*a).try_clone()?;
        a.try_push_str(&b)?;
        Ok(Shared::new(a).with_span(span)?)
    }
}

fn eval_ir_branches(
    ir: &ir::IrBranches,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    for (ir_condition, branch) in &ir.branches {
        let guard = interp.scopes.push()?;

        let value = eval_ir_condition(ir_condition, interp, used)?;

        let output = if as_bool(ir_condition.span(), value)? {
            Some(eval_ir_scope(branch, interp, used)?)
        } else {
            None
        };

        interp.scopes.pop(guard).with_span(branch)?;

        if let Some(output) = output {
            return Ok(output);
        }
    }

    if let Some(branch) = &ir.default_branch {
        return eval_ir_scope(branch, interp, used);
    }

    Ok(ir::Value::EmptyTuple)
}

fn eval_ir_call(
    ir: &ir::IrCall,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    let mut args = Vec::new();

    for arg in &ir.args {
        args.try_push(eval_ir(arg, interp, used)?)?;
    }

    Ok(interp.call_const_fn(ir, ir.id, args, used)?)
}

fn eval_ir_condition(
    ir: &ir::IrCondition,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    Ok(ir::Value::Bool(match ir {
        ir::IrCondition::Ir(ir) => {
            let value = eval_ir(ir, interp, used)?;
            as_bool(ir.span(), value)?
        }
        ir::IrCondition::Let(ir_let) => {
            let value = eval_ir(&ir_let.ir, interp, used)?;
            ir_let.pat.matches(interp, value, ir)?
        }
    }))
}

fn eval_ir_decl(
    ir: &ir::IrDecl,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    interp.budget.take(ir)?;
    let value = eval_ir(&ir.value, interp, used)?;
    interp.scopes.decl(&ir.name, value).with_span(ir)?;
    Ok(ir::Value::EmptyTuple)
}

fn eval_ir_loop(
    ir: &ir::IrLoop,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    let span = ir.span();
    interp.budget.take(span)?;

    let guard = interp.scopes.push()?;

    let value = loop {
        if let Some(condition) = &ir.condition {
            interp.scopes.clear_current().with_span(condition)?;

            let value = eval_ir_condition(condition, interp, used)?;

            if !as_bool(condition.span(), value)? {
                break None;
            }
        }

        match eval_ir_scope(&ir.body, interp, used) {
            Ok(..) => (),
            Err(outcome) => match outcome {
                EvalOutcome::Break(span, label, expr) => {
                    if label.as_deref() == ir.label.as_deref() {
                        break expr;
                    } else {
                        return Err(EvalOutcome::Break(span, label, expr));
                    }
                }
                outcome => return Err(outcome),
            },
        };
    };

    interp.scopes.pop(guard).with_span(ir)?;

    if let Some(value) = value {
        if ir.condition.is_some() {
            return Err(EvalOutcome::from(compile::Error::msg(
                span,
                "break with value is not supported for unconditional loops",
            )));
        }

        Ok(value)
    } else {
        Ok(ir::Value::EmptyTuple)
    }
}

fn eval_ir_object(
    ir: &ir::IrObject,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    let mut object = HashMap::try_with_capacity(ir.assignments.len())?;

    for (key, value) in ir.assignments.iter() {
        object.try_insert(key.as_ref().try_to_owned()?, eval_ir(value, interp, used)?)?;
    }

    Ok(ir::Value::Object(Shared::new(object).with_span(ir)?))
}

fn eval_ir_scope(
    ir: &ir::IrScope,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    interp.budget.take(ir)?;
    let guard = interp.scopes.push()?;

    for ir in &ir.instructions {
        let _ = eval_ir(ir, interp, used)?;
    }

    let value = if let Some(last) = &ir.last {
        eval_ir(last, interp, used)?
    } else {
        ir::Value::EmptyTuple
    };

    interp.scopes.pop(guard).with_span(ir)?;
    Ok(value)
}

fn eval_ir_set(
    ir: &ir::IrSet,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    interp.budget.take(ir)?;
    let value = eval_ir(&ir.value, interp, used)?;
    interp.scopes.set_target(&ir.target, value)?;
    Ok(ir::Value::EmptyTuple)
}

fn eval_ir_template(
    ir: &ir::IrTemplate,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    interp.budget.take(ir)?;

    let mut buf = String::new();

    for component in &ir.components {
        match component {
            ir::IrTemplateComponent::String(string) => {
                buf.try_push_str(string)?;
            }
            ir::IrTemplateComponent::Ir(ir) => {
                let const_value = eval_ir(ir, interp, used)?;

                match const_value {
                    ir::Value::Integer(integer) => {
                        write!(buf, "{}", integer)?;
                    }
                    ir::Value::Float(float) => {
                        let mut buffer = ryu::Buffer::new();
                        buf.try_push_str(buffer.format(float))?;
                    }
                    ir::Value::Bool(b) => {
                        write!(buf, "{}", b)?;
                    }
                    ir::Value::String(s) => {
                        let s = s.borrow_ref().with_span(ir)?;
                        buf.try_push_str(&s)?;
                    }
                    _ => {
                        return Err(EvalOutcome::not_const(ir));
                    }
                }
            }
        }
    }

    Ok(ir::Value::String(Shared::new(buf).with_span(ir)?))
}

fn eval_ir_tuple(
    ir: &ir::Tuple,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    let mut items = Vec::try_with_capacity(ir.items.len())?;

    for item in ir.items.iter() {
        items.try_push(eval_ir(item, interp, used)?)?;
    }

    Ok(ir::Value::Tuple(
        Shared::new(items.try_into_boxed_slice()?).with_span(ir)?,
    ))
}

fn eval_ir_vec(
    ir: &ir::IrVec,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    let mut vec = Vec::try_with_capacity(ir.items.len())?;

    for item in ir.items.iter() {
        vec.try_push(eval_ir(item, interp, used)?)?;
    }

    Ok(ir::Value::Vec(Shared::new(vec).with_span(ir)?))
}

/// IrEval the interior expression.
pub(crate) fn eval_ir(
    ir: &ir::Ir,
    interp: &mut ir::Interpreter<'_, '_>,
    used: Used,
) -> Result<ir::Value, EvalOutcome> {
    interp.budget.take(ir)?;

    match &ir.kind {
        ir::IrKind::Scope(ir) => eval_ir_scope(ir, interp, used),
        ir::IrKind::Binary(ir) => eval_ir_binary(ir, interp, used),
        ir::IrKind::Decl(ir) => eval_ir_decl(ir, interp, used),
        ir::IrKind::Set(ir) => eval_ir_set(ir, interp, used),
        ir::IrKind::Assign(ir) => eval_ir_assign(ir, interp, used),
        ir::IrKind::Template(ir) => eval_ir_template(ir, interp, used),
        ir::IrKind::Name(name) => Ok(interp.resolve_var(ir, name, used)?),
        ir::IrKind::Target(target) => Ok(interp.scopes.get_target(target)?),
        ir::IrKind::Value(value) => Ok(value.try_clone()?),
        ir::IrKind::Branches(ir) => eval_ir_branches(ir, interp, used),
        ir::IrKind::Loop(ir) => eval_ir_loop(ir, interp, used),
        ir::IrKind::Break(ir) => Err(ir.as_outcome(interp, used)),
        ir::IrKind::Vec(ir) => eval_ir_vec(ir, interp, used),
        ir::IrKind::Tuple(ir) => eval_ir_tuple(ir, interp, used),
        ir::IrKind::Object(ir) => eval_ir_object(ir, interp, used),
        ir::IrKind::Call(ir) => eval_ir_call(ir, interp, used),
    }
}
