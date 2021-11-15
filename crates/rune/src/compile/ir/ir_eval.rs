use crate::ast::{Span, Spanned};
use crate::collections::HashMap;
use crate::compile::ir;
use crate::compile::ir::{IrError, IrInterpreter, IrValue};
use crate::query::Used;
use crate::runtime::Shared;
use std::convert::TryFrom;
use std::fmt::Write;

/// The trait for something that can be evaluated in a constant context.
pub trait IrEval {
    /// Evaluate the given type.
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome>;

    /// Process constant value as a boolean.
    fn eval_bool(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<bool, IrEvalOutcome>
    where
        Self: Spanned,
    {
        let span = self.span();

        let value = self
            .eval(interp, used)?
            .into_bool()
            .map_err(|actual| IrError::expected::<_, bool>(span, &actual))?;

        Ok(value)
    }
}

/// The outcome of a constant evaluation.
pub enum IrEvalOutcome {
    /// Encountered expression that is not a valid constant expression.
    NotConst(Span),
    /// A compile error.
    Error(IrError),
    /// Break until the next loop, or the optional label.
    Break(Span, IrEvalBreak),
}

impl IrEvalOutcome {
    /// Encountered ast that is not a constant expression.
    pub(crate) fn not_const<S>(spanned: S) -> Self
    where
        S: Spanned,
    {
        Self::NotConst(spanned.span())
    }
}

impl<T> From<T> for IrEvalOutcome
where
    IrError: From<T>,
{
    fn from(error: T) -> Self {
        Self::Error(IrError::from(error))
    }
}

/// The value of a break.
pub enum IrEvalBreak {
    /// Break the next nested loop.
    Inherent,
    /// The break had a value.
    Value(IrValue),
    /// The break had a label.
    Label(Box<str>),
}

impl IrEval for ir::IrAssign {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        interp.budget.take(self)?;
        let value = self.value.eval(interp, used)?;

        interp
            .scopes
            .mut_target(&self.target, move |t| self.op.assign(self, t, value))?;

        Ok(IrValue::Unit)
    }
}

impl IrEval for ir::IrBinary {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
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
                        .ok_or_else(|| IrError::msg(span, "division by zero"))?;
                    return Ok(IrValue::Integer(number));
                }
                ir::IrBinaryOp::Shl => {
                    let b = u32::try_from(b).map_err(|_| {
                        IrError::msg(&self.rhs, "cannot be converted to shift operand")
                    })?;

                    let n = a.shl(b);
                    return Ok(IrValue::Integer(n));
                }
                ir::IrBinaryOp::Shr => {
                    let b = u32::try_from(b).map_err(|_| {
                        IrError::msg(&self.rhs, "cannot be converted to shift operand")
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
                #[allow(clippy::float_cmp)]
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
            (IrValue::String(a), IrValue::String(b)) => {
                if let ir::IrBinaryOp::Add = self.op {
                    return Ok(IrValue::String(add_strings(span, &a, &b)?));
                }
            }
            _ => (),
        }

        return Err(IrEvalOutcome::not_const(span));

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
    }
}

impl IrEval for ir::IrBranches {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        for (ir_condition, branch) in &self.branches {
            let guard = interp.scopes.push();

            let output = if ir_condition.eval_bool(interp, used)? {
                Some(branch.eval(interp, used)?)
            } else {
                None
            };

            interp.scopes.pop(branch, guard)?;

            if let Some(output) = output {
                return Ok(output);
            }
        }

        if let Some(branch) = &self.default_branch {
            return branch.eval(interp, used);
        }

        Ok(IrValue::Unit)
    }
}

impl IrEval for ir::IrCall {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        let mut args = Vec::new();

        for arg in &self.args {
            args.push(arg.eval(interp, used)?);
        }

        Ok(interp.call_const_fn(self, &self.target, args, used)?)
    }
}

impl IrEval for ir::IrCondition {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        Ok(IrValue::Bool(match self {
            ir::IrCondition::Ir(ir) => ir.eval_bool(interp, used)?,
            ir::IrCondition::Let(ir_let) => {
                let value = ir_let.ir.eval(interp, used)?;
                ir_let.pat.matches(interp, value, self)?
            }
        }))
    }
}

impl IrEval for ir::IrDecl {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        interp.budget.take(self)?;
        let value = self.value.eval(interp, used)?;
        interp.scopes.decl(&self.name, value, self)?;
        Ok(IrValue::Unit)
    }
}

impl IrEval for ir::IrLoop {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        let span = self.span();
        interp.budget.take(span)?;

        let guard = interp.scopes.push();

        loop {
            if let Some(condition) = &self.condition {
                interp.scopes.clear_current(&*condition)?;

                if !condition.eval_bool(interp, used)? {
                    break;
                }
            }

            match self.body.eval(interp, used) {
                Ok(..) => (),
                Err(outcome) => match outcome {
                    IrEvalOutcome::Break(span, b) => match b {
                        IrEvalBreak::Inherent => break,
                        IrEvalBreak::Label(l) => {
                            if self.label.as_ref() == Some(&l) {
                                break;
                            }

                            return Err(IrEvalOutcome::Break(span, IrEvalBreak::Label(l)));
                        }
                        IrEvalBreak::Value(value) => {
                            if self.condition.is_none() {
                                return Ok(value);
                            }

                            return Err(IrEvalOutcome::from(IrError::msg(
                                span,
                                "break with value is not supported for unconditional loops",
                            )));
                        }
                    },
                    outcome => return Err(outcome),
                },
            };
        }

        interp.scopes.pop(self, guard)?;
        Ok(IrValue::Unit)
    }
}

impl IrEval for ir::IrObject {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        let mut object = HashMap::with_capacity(self.assignments.len());

        for (key, value) in self.assignments.iter() {
            object.insert(key.as_ref().to_owned(), value.eval(interp, used)?);
        }

        Ok(IrValue::Object(Shared::new(object)))
    }
}

impl IrEval for ir::IrScope {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        interp.budget.take(self)?;
        let guard = interp.scopes.push();

        for ir in &self.instructions {
            let _ = ir.eval(interp, used)?;
        }

        let value = if let Some(last) = &self.last {
            last.eval(interp, used)?
        } else {
            IrValue::Unit
        };

        interp.scopes.pop(self, guard)?;
        Ok(value)
    }
}

impl IrEval for ir::IrSet {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        interp.budget.take(self)?;
        let value = self.value.eval(interp, used)?;
        interp.scopes.set_target(&self.target, value)?;
        Ok(IrValue::Unit)
    }
}

impl IrEval for ir::IrTemplate {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        interp.budget.take(self)?;

        let mut buf = String::new();

        for component in &self.components {
            match component {
                ir::IrTemplateComponent::String(string) => {
                    buf.push_str(string);
                }
                ir::IrTemplateComponent::Ir(ir) => {
                    let const_value = ir.eval(interp, used)?;

                    match const_value {
                        IrValue::Integer(integer) => {
                            write!(buf, "{}", integer).unwrap();
                        }
                        IrValue::Float(float) => {
                            let mut buffer = ryu::Buffer::new();
                            buf.push_str(buffer.format(float));
                        }
                        IrValue::Bool(b) => {
                            write!(buf, "{}", b).unwrap();
                        }
                        IrValue::String(s) => {
                            let s = s.borrow_ref().map_err(IrError::access(self))?;
                            buf.push_str(&*s);
                        }
                        _ => {
                            return Err(IrEvalOutcome::not_const(self));
                        }
                    }
                }
            }
        }

        Ok(IrValue::String(Shared::new(buf)))
    }
}

impl IrEval for ir::IrTuple {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        let mut items = Vec::with_capacity(self.items.len());

        for item in self.items.iter() {
            items.push(item.eval(interp, used)?);
        }

        Ok(IrValue::Tuple(Shared::new(items.into_boxed_slice())))
    }
}

impl IrEval for ir::IrVec {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        let mut vec = Vec::with_capacity(self.items.len());

        for item in self.items.iter() {
            vec.push(item.eval(interp, used)?);
        }

        Ok(IrValue::Vec(Shared::new(vec)))
    }
}

/// IrEval the interior expression.
impl IrEval for ir::Ir {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        interp.budget.take(self)?;

        match &self.kind {
            ir::IrKind::Scope(ir_scope) => ir_scope.eval(interp, used),
            ir::IrKind::Binary(ir_binary) => ir_binary.eval(interp, used),
            ir::IrKind::Decl(ir_decl) => ir_decl.eval(interp, used),
            ir::IrKind::Set(ir_set) => ir_set.eval(interp, used),
            ir::IrKind::Assign(ir_assign) => ir_assign.eval(interp, used),
            ir::IrKind::Template(ir_template) => ir_template.eval(interp, used),
            ir::IrKind::Name(name) => Ok(interp.resolve_var(self.span(), name.as_ref(), used)?),
            ir::IrKind::Target(ir_target) => Ok(interp.scopes.get_target(ir_target)?),
            ir::IrKind::Value(value) => Ok(value.clone()),
            ir::IrKind::Branches(branches) => branches.eval(interp, used),
            ir::IrKind::Loop(ir_loop) => ir_loop.eval(interp, used),
            ir::IrKind::Break(ir_break) => Err(ir_break.as_outcome(interp, used)),
            ir::IrKind::Vec(ir_vec) => ir_vec.eval(interp, used),
            ir::IrKind::Tuple(ir_tuple) => ir_tuple.eval(interp, used),
            ir::IrKind::Object(ir_object) => ir_object.eval(interp, used),
            ir::IrKind::Call(ir_call) => ir_call.eval(interp, used),
        }
    }
}
