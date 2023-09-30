use crate::alloc::prelude::*;
use crate::alloc::{try_format, Vec};
use crate::ast::Spanned;
use crate::compile::ir;
use crate::compile::ir::scopes::MissingLocal;
use crate::compile::meta;
use crate::compile::{self, IrErrorKind, ItemId, ModId, WithSpan};
use crate::hir;
use crate::parse::NonZeroId;
use crate::query::{Query, Used};
use crate::runtime::{ConstValue, Object, OwnedTuple};

/// The interpreter that executed [Ir][crate::ir::Ir].
pub struct Interpreter<'a, 'arena> {
    /// A budget associated with the compiler, for how many expressions it's
    /// allowed to evaluate.
    pub(crate) budget: Budget,
    /// The module in which the interpreter is run.
    pub(crate) module: ModId,
    /// The item where the constant expression is located.
    pub(crate) item: ItemId,
    /// Constant scopes.
    pub(crate) scopes: ir::Scopes,
    /// Query engine to look for constant expressions.
    pub(crate) q: Query<'a, 'arena>,
}

impl Interpreter<'_, '_> {
    /// Outer evaluation for an expression which performs caching into `consts`.
    pub(crate) fn eval_const(&mut self, ir: &ir::Ir, used: Used) -> compile::Result<ConstValue> {
        tracing::trace!("processing constant: {}", self.q.pool.item(self.item));

        if let Some(const_value) = self.q.consts.get(self.item) {
            return Ok(const_value.try_clone()?);
        }

        if !self.q.consts.mark(self.item)? {
            return Err(compile::Error::new(ir, IrErrorKind::ConstCycle));
        }

        let ir_value = match ir::eval_ir(ir, self, used) {
            Ok(ir_value) => ir_value,
            Err(outcome) => match outcome {
                ir::EvalOutcome::Error(error) => {
                    return Err(error);
                }
                ir::EvalOutcome::NotConst(span) => {
                    return Err(compile::Error::new(span, IrErrorKind::NotConst))
                }
                ir::EvalOutcome::Break(span, _, _) => {
                    return Err(compile::Error::new(span, IrErrorKind::BreakOutsideOfLoop))
                }
            },
        };

        let const_value = ir_value.into_const(ir)?;

        if self
            .q
            .consts
            .insert(self.item, const_value.try_clone()?)?
            .is_some()
        {
            return Err(compile::Error::new(ir, IrErrorKind::ConstCycle));
        }

        Ok(const_value)
    }

    /// Evaluate to an ir value.
    pub(crate) fn eval_value(&mut self, ir: &ir::Ir, used: Used) -> compile::Result<ir::Value> {
        match ir::eval_ir(ir, self, used) {
            Ok(ir_value) => Ok(ir_value),
            Err(outcome) => match outcome {
                ir::EvalOutcome::Error(error) => Err(error),
                ir::EvalOutcome::NotConst(span) => {
                    Err(compile::Error::new(span, IrErrorKind::NotConst))
                }
                ir::EvalOutcome::Break(span, _, _) => {
                    Err(compile::Error::new(span, IrErrorKind::BreakOutsideOfLoop))
                }
            },
        }
    }

    /// Resolve the given constant value from the block scope.
    ///
    /// This looks up `const <ident> = <expr>` and evaluates them while caching
    /// their result.
    pub(crate) fn resolve_var(
        &mut self,
        span: &dyn Spanned,
        name: &hir::OwnedName,
        used: Used,
    ) -> compile::Result<ir::Value> {
        if let Some(ir_value) = self.scopes.try_get(name) {
            return Ok(ir_value.try_clone()?);
        }

        let mut base = self.q.pool.item(self.item).try_to_owned()?;

        loop {
            let item = self
                .q
                .pool
                .alloc_item(base.extended(name.try_to_string()?)?)?;

            if let Some(const_value) = self.q.consts.get(item) {
                return Ok(ir::Value::from_const(const_value).with_span(span)?);
            }

            if let Some(meta) = self.q.query_meta(span, item, used)? {
                match &meta.kind {
                    meta::Kind::Const => {
                        let Some(const_value) = self.q.get_const_value(meta.hash) else {
                            return Err(compile::Error::msg(
                                span,
                                try_format!("Missing constant for hash {}", meta.hash),
                            ));
                        };

                        return Ok(ir::Value::from_const(const_value).with_span(span)?);
                    }
                    _ => {
                        return Err(compile::Error::new(
                            span,
                            IrErrorKind::UnsupportedMeta {
                                meta: meta.info(self.q.pool)?,
                            },
                        ));
                    }
                }
            }

            if base.is_empty() {
                break;
            }

            base.pop()?;
        }

        if name.as_ref().starts_with(char::is_lowercase) {
            Err(compile::Error::new(
                span,
                MissingLocal(name.try_to_string()?.try_into_boxed_str()?),
            ))
        } else {
            Err(compile::Error::new(
                span,
                IrErrorKind::MissingConst {
                    name: name.try_to_string()?.try_into()?,
                },
            ))
        }
    }

    pub(crate) fn call_const_fn<S>(
        &mut self,
        spanned: S,
        id: NonZeroId,
        args: Vec<ir::Value>,
        used: Used,
    ) -> compile::Result<ir::Value>
    where
        S: Copy + Spanned,
    {
        let span = Spanned::span(&spanned);
        let const_fn = self.q.const_fn_for(id).with_span(span)?;

        if const_fn.ir_fn.args.len() != args.len() {
            return Err(compile::Error::new(
                span,
                IrErrorKind::ArgumentCountMismatch {
                    actual: args.len(),
                    expected: const_fn.ir_fn.args.len(),
                },
            ));
        }

        let guard = self.scopes.isolate()?;

        for (name, value) in const_fn.ir_fn.args.iter().zip(args) {
            self.scopes.decl(name, value).with_span(span)?;
        }

        let value = self.eval_value(&const_fn.ir_fn.ir, used)?;
        self.scopes.pop(guard).with_span(span)?;
        Ok(value)
    }
}

impl ir::Scopes {
    /// Get the given target as mut.
    pub(crate) fn get_target(&mut self, ir_target: &ir::IrTarget) -> compile::Result<ir::Value> {
        match &ir_target.kind {
            ir::IrTargetKind::Name(name) => Ok(self.get_name(name, ir_target)?.try_clone()?),
            ir::IrTargetKind::Field(ir_target, field) => {
                let value = self.get_target(ir_target)?;

                match value {
                    ir::Value::Object(object) => {
                        let object = object.borrow_ref().with_span(ir_target)?;

                        if let Some(value) = object.get(field.as_ref()).try_cloned()? {
                            return Ok(value);
                        }
                    }
                    actual => {
                        return Err(compile::Error::expected_type::<_, OwnedTuple>(
                            ir_target, &actual,
                        ))
                    }
                };

                Err(compile::Error::new(
                    ir_target,
                    IrErrorKind::MissingField {
                        field: field.try_clone()?,
                    },
                ))
            }
            ir::IrTargetKind::Index(target, index) => {
                let value = self.get_target(target)?;

                match value {
                    ir::Value::Vec(vec) => {
                        let vec = vec.borrow_ref().with_span(ir_target)?;

                        if let Some(value) = vec.get(*index).try_cloned()? {
                            return Ok(value);
                        }
                    }
                    ir::Value::Tuple(tuple) => {
                        let tuple = tuple.borrow_ref().with_span(ir_target)?;

                        if let Some(value) = tuple.get(*index).try_cloned()? {
                            return Ok(value);
                        }
                    }
                    actual => {
                        return Err(compile::Error::expected_type::<_, OwnedTuple>(
                            ir_target, &actual,
                        ))
                    }
                };

                Err(compile::Error::new(
                    ir_target,
                    IrErrorKind::MissingIndex { index: *index },
                ))
            }
        }
    }

    /// Update the given target with the given constant value.
    pub(crate) fn set_target(
        &mut self,
        ir_target: &ir::IrTarget,
        value: ir::Value,
    ) -> compile::Result<()> {
        match &ir_target.kind {
            ir::IrTargetKind::Name(name) => {
                *self.get_name_mut(name, ir_target)? = value;
                Ok(())
            }
            ir::IrTargetKind::Field(target, field) => {
                let current = self.get_target(target)?;

                match current {
                    ir::Value::Object(object) => {
                        let mut object = object.borrow_mut().with_span(ir_target)?;
                        object.try_insert(field.as_ref().try_to_owned()?, value)?;
                    }
                    actual => {
                        return Err(compile::Error::expected_type::<_, Object>(
                            ir_target, &actual,
                        ));
                    }
                }

                Ok(())
            }
            ir::IrTargetKind::Index(target, index) => {
                let current = self.get_target(target)?;

                match current {
                    ir::Value::Vec(vec) => {
                        let mut vec = vec.borrow_mut().with_span(ir_target)?;

                        if let Some(current) = vec.get_mut(*index) {
                            *current = value;
                            return Ok(());
                        }
                    }
                    ir::Value::Tuple(tuple) => {
                        let mut tuple = tuple.borrow_mut().with_span(ir_target)?;

                        if let Some(current) = tuple.get_mut(*index) {
                            *current = value;
                            return Ok(());
                        }
                    }
                    actual => {
                        return Err(compile::Error::expected_type::<_, OwnedTuple>(
                            ir_target, &actual,
                        ));
                    }
                };

                Err(compile::Error::msg(ir_target, "missing index"))
            }
        }
    }

    /// Mutate the given target with the given constant value.
    pub(crate) fn mut_target(
        &mut self,
        ir_target: &ir::IrTarget,
        op: impl FnOnce(&mut ir::Value) -> compile::Result<()>,
    ) -> compile::Result<()> {
        match &ir_target.kind {
            ir::IrTargetKind::Name(name) => {
                let value = self.get_name_mut(name, ir_target)?;
                op(value)
            }
            ir::IrTargetKind::Field(target, field) => {
                let current = self.get_target(target)?;

                match current {
                    ir::Value::Object(object) => {
                        let mut object = object.borrow_mut().with_span(ir_target)?;

                        let Some(value) = object.get_mut(field.as_ref()) else {
                            return Err(compile::Error::new(
                                ir_target,
                                IrErrorKind::MissingField {
                                    field: field.try_clone()?,
                                },
                            ));
                        };

                        op(value)
                    }
                    actual => Err(compile::Error::expected_type::<_, Object>(
                        ir_target, &actual,
                    )),
                }
            }
            ir::IrTargetKind::Index(target, index) => {
                let current = self.get_target(target)?;

                match current {
                    ir::Value::Vec(vec) => {
                        let mut vec = vec.borrow_mut().with_span(ir_target)?;

                        let value = vec.get_mut(*index).ok_or_else(|| {
                            compile::Error::new(
                                ir_target,
                                IrErrorKind::MissingIndex { index: *index },
                            )
                        })?;

                        op(value)
                    }
                    ir::Value::Tuple(tuple) => {
                        let mut tuple = tuple.borrow_mut().with_span(ir_target)?;

                        let value = tuple.get_mut(*index).ok_or_else(|| {
                            compile::Error::new(
                                ir_target,
                                IrErrorKind::MissingIndex { index: *index },
                            )
                        })?;

                        op(value)
                    }
                    actual => Err(compile::Error::expected_type::<_, OwnedTuple>(
                        ir_target, &actual,
                    )),
                }
            }
        }
    }
}

/// A budget dictating the number of evaluations the compiler is allowed to do.
pub(crate) struct Budget {
    budget: usize,
}

impl Budget {
    /// Construct a new constant evaluation budget with the given constraint.
    pub(crate) fn new(budget: usize) -> Self {
        Self { budget }
    }

    /// Take an item from the budget. Errors if the budget is exceeded.
    pub(crate) fn take<S>(&mut self, spanned: S) -> compile::Result<()>
    where
        S: Spanned,
    {
        if self.budget == 0 {
            return Err(compile::Error::new(spanned, IrErrorKind::BudgetExceeded));
        }

        self.budget -= 1;
        Ok(())
    }
}
