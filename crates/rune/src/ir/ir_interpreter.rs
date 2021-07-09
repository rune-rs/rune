use crate::ir;
use crate::ir::eval::{IrEval, IrEvalOutcome};
use crate::ir::{IrQuery, IrValue};
use crate::query::Used;
use crate::shared::Consts;
use crate::{IrError, IrErrorKind, Spanned};
use runestick::{CompileMetaKind, CompileMod, ConstValue, Item, Span};
use std::sync::Arc;

/// Ir Scopes.
pub(crate) type IrScopes = crate::shared::Scopes<IrValue>;

/// The interpreter that executed [Ir][crate::ir::Ir].
#[allow(dead_code)]
pub struct IrInterpreter<'a> {
    /// A budget associated with the compiler, for how many expressions it's
    /// allowed to evaluate.
    pub(crate) budget: IrBudget,
    /// The module in which the interpreter is run.
    pub(crate) module: Arc<CompileMod>,
    /// The item where the constant expression is located.
    pub(crate) item: Item,
    /// Constant scopes.
    pub(crate) scopes: IrScopes,
    /// Constant values.
    pub(crate) consts: Consts,
    /// Query engine to look for constant expressions.
    pub(crate) query: &'a mut dyn IrQuery,
}

impl IrInterpreter<'_> {
    /// Evaluate the given target.
    pub(crate) fn eval<T>(&mut self, target: &T, used: Used) -> Result<T::Output, IrEvalOutcome>
    where
        T: IrEval,
    {
        target.eval(self, used)
    }

    /// Outer evaluation for an expression which performs caching into `consts`.
    pub(crate) fn eval_const(&mut self, ir: &ir::Ir, used: Used) -> Result<ConstValue, IrError> {
        log::trace!("processing constant: {}", self.item);

        if let Some(const_value) = self.consts.get(&self.item) {
            return Ok(const_value);
        }

        if !self.consts.mark(&self.item) {
            return Err(IrError::new(ir, IrErrorKind::ConstCycle));
        }

        let ir_value = match self.eval(ir, used) {
            Ok(ir_value) => ir_value,
            Err(outcome) => match outcome {
                IrEvalOutcome::Error(error) => {
                    return Err(error);
                }
                IrEvalOutcome::NotConst(span) => {
                    return Err(IrError::new(span, IrErrorKind::NotConst))
                }
                IrEvalOutcome::Break(span, _) => {
                    return Err(IrError::new(span, IrErrorKind::BreakOutsideOfLoop))
                }
            },
        };

        let const_value = ir_value.into_const(ir)?;

        if self
            .consts
            .insert(self.item.clone(), const_value.clone())
            .is_some()
        {
            return Err(IrError::new(ir, IrErrorKind::ConstCycle));
        }

        Ok(const_value)
    }

    /// Evaluate to an ir value.
    pub(crate) fn eval_value(&mut self, ir: &ir::Ir, used: Used) -> Result<IrValue, IrError> {
        match self.eval(ir, used) {
            Ok(ir_value) => Ok(ir_value),
            Err(outcome) => match outcome {
                IrEvalOutcome::Error(error) => Err(error),
                IrEvalOutcome::NotConst(span) => Err(IrError::new(span, IrErrorKind::NotConst)),
                IrEvalOutcome::Break(span, _) => {
                    Err(IrError::new(span, IrErrorKind::BreakOutsideOfLoop))
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
        spanned: Span,
        name: &str,
        used: Used,
    ) -> Result<IrValue, IrError> {
        if let Some(ir_value) = self.scopes.try_get(name) {
            return Ok(ir_value.clone());
        }

        let mut base = self.item.clone();

        loop {
            let item = base.extended(name);

            if let Some(const_value) = self.consts.get(&item) {
                return Ok(IrValue::from_const(const_value));
            }

            if let Some(meta) = self.query.query_meta(spanned, &item, used)? {
                match &meta.kind {
                    CompileMetaKind::Const { const_value, .. } => {
                        return Ok(IrValue::from_const(const_value.clone()));
                    }
                    _ => {
                        return Err(IrError::new(spanned, IrErrorKind::UnsupportedMeta { meta }));
                    }
                }
            }

            if base.is_empty() {
                break;
            }

            base.pop();
        }

        if name.starts_with(char::is_lowercase) {
            Err(IrError::new(
                spanned,
                IrErrorKind::MissingLocal { name: name.into() },
            ))
        } else {
            Err(IrError::new(
                spanned,
                IrErrorKind::MissingConst { name: name.into() },
            ))
        }
    }

    pub(crate) fn call_const_fn<S>(
        &mut self,
        spanned: S,
        target: &str,
        args: Vec<IrValue>,
        used: Used,
    ) -> Result<IrValue, IrError>
    where
        S: Copy + Spanned,
    {
        let span = spanned.span();
        let mut base = self.item.clone();

        let id = loop {
            let item = base.extended(target);

            if let Some(meta) = self.query.query_meta(span, &item, used)? {
                match &meta.kind {
                    CompileMetaKind::ConstFn { id, .. } => {
                        break *id;
                    }
                    _ => {
                        return Err(IrError::new(span, IrErrorKind::UnsupportedMeta { meta }));
                    }
                }
            }

            if base.is_empty() {
                return Err(IrError::new(spanned, IrErrorKind::FnNotFound));
            }

            base.pop();
        };

        let const_fn = self.query.const_fn_for(spanned.span(), Some(id))?;

        if const_fn.ir_fn.args.len() != args.len() {
            return Err(IrError::new(
                spanned,
                IrErrorKind::ArgumentCountMismatch {
                    actual: args.len(),
                    expected: const_fn.ir_fn.args.len(),
                },
            ));
        }

        let guard = self.scopes.isolate();

        for (name, value) in const_fn.ir_fn.args.iter().zip(args) {
            self.scopes.decl(&**name, value, spanned)?;
        }

        let value = self.eval_value(&const_fn.ir_fn.ir, used)?;
        self.scopes.pop(spanned, guard)?;
        Ok(value)
    }
}

impl IrScopes {
    /// Get the given target as mut.
    pub(crate) fn get_target(&mut self, ir_target: &ir::IrTarget) -> Result<IrValue, IrError> {
        match &ir_target.kind {
            ir::IrTargetKind::Name(name) => Ok(self.get_name(name, ir_target)?.clone()),
            ir::IrTargetKind::Field(ir_target, field) => {
                let value = self.get_target(ir_target)?;

                match value {
                    IrValue::Object(object) => {
                        let object = object.borrow_ref().map_err(IrError::access(ir_target))?;

                        if let Some(value) = object.get(field.as_ref()).cloned() {
                            return Ok(value);
                        }
                    }
                    actual => {
                        return Err(IrError::expected::<_, runestick::Tuple>(ir_target, &actual))
                    }
                };

                Err(IrError::new(
                    ir_target,
                    IrErrorKind::MissingField {
                        field: field.clone(),
                    },
                ))
            }
            ir::IrTargetKind::Index(target, index) => {
                let value = self.get_target(target)?;

                match value {
                    IrValue::Vec(vec) => {
                        let vec = vec.borrow_ref().map_err(IrError::access(ir_target))?;

                        if let Some(value) = vec.get(*index).cloned() {
                            return Ok(value);
                        }
                    }
                    IrValue::Tuple(tuple) => {
                        let tuple = tuple.borrow_ref().map_err(IrError::access(ir_target))?;

                        if let Some(value) = tuple.get(*index).cloned() {
                            return Ok(value);
                        }
                    }
                    actual => {
                        return Err(IrError::expected::<_, runestick::Tuple>(ir_target, &actual))
                    }
                };

                Err(IrError::new(
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
        value: IrValue,
    ) -> Result<(), IrError> {
        match &ir_target.kind {
            ir::IrTargetKind::Name(name) => {
                *self.get_name_mut(name.as_ref(), ir_target)? = value;
                Ok(())
            }
            ir::IrTargetKind::Field(target, field) => {
                let current = self.get_target(target)?;

                match current {
                    IrValue::Object(object) => {
                        let mut object = object.borrow_mut().map_err(IrError::access(ir_target))?;
                        object.insert(field.as_ref().to_owned(), value);
                    }
                    actual => {
                        return Err(IrError::expected::<_, runestick::Object>(
                            ir_target, &actual,
                        ));
                    }
                }

                Ok(())
            }
            ir::IrTargetKind::Index(target, index) => {
                let current = self.get_target(target)?;

                match current {
                    IrValue::Vec(vec) => {
                        let mut vec = vec.borrow_mut().map_err(IrError::access(ir_target))?;

                        if let Some(current) = vec.get_mut(*index) {
                            *current = value;
                            return Ok(());
                        }
                    }
                    IrValue::Tuple(tuple) => {
                        let mut tuple = tuple.borrow_mut().map_err(IrError::access(ir_target))?;

                        if let Some(current) = tuple.get_mut(*index) {
                            *current = value;
                            return Ok(());
                        }
                    }
                    actual => {
                        return Err(IrError::expected::<_, runestick::Tuple>(ir_target, &actual));
                    }
                };

                Err(IrError::msg(ir_target, "missing index"))
            }
        }
    }

    /// Mutate the given target with the given constant value.
    pub(crate) fn mut_target(
        &mut self,
        ir_target: &ir::IrTarget,
        op: impl FnOnce(&mut IrValue) -> Result<(), IrError>,
    ) -> Result<(), IrError> {
        match &ir_target.kind {
            ir::IrTargetKind::Name(name) => {
                let value = self.get_name_mut(name.as_ref(), ir_target)?;
                op(value)
            }
            ir::IrTargetKind::Field(target, field) => {
                let current = self.get_target(target)?;

                match current {
                    IrValue::Object(object) => {
                        let mut object = object.borrow_mut().map_err(IrError::access(ir_target))?;

                        let value = object.get_mut(field.as_ref()).ok_or_else(|| {
                            IrError::new(
                                ir_target,
                                IrErrorKind::MissingField {
                                    field: field.clone(),
                                },
                            )
                        })?;

                        op(value)
                    }
                    actual => Err(IrError::expected::<_, runestick::Object>(
                        ir_target, &actual,
                    )),
                }
            }
            ir::IrTargetKind::Index(target, index) => {
                let current = self.get_target(target)?;

                match current {
                    IrValue::Vec(vec) => {
                        let mut vec = vec.borrow_mut().map_err(IrError::access(ir_target))?;

                        let value = vec.get_mut(*index).ok_or_else(|| {
                            IrError::new(ir_target, IrErrorKind::MissingIndex { index: *index })
                        })?;

                        op(value)
                    }
                    IrValue::Tuple(tuple) => {
                        let mut tuple = tuple.borrow_mut().map_err(IrError::access(ir_target))?;

                        let value = tuple.get_mut(*index).ok_or_else(|| {
                            IrError::new(ir_target, IrErrorKind::MissingIndex { index: *index })
                        })?;

                        op(value)
                    }
                    actual => Err(IrError::expected::<_, runestick::Tuple>(ir_target, &actual)),
                }
            }
        }
    }
}

/// A budget dictating the number of evaluations the compiler is allowed to do.
pub(crate) struct IrBudget {
    budget: usize,
}

impl IrBudget {
    /// Construct a new constant evaluation budget with the given constraint.
    pub(crate) fn new(budget: usize) -> Self {
        Self { budget }
    }

    /// Take an item from the budget. Errors if the budget is exceeded.
    pub(crate) fn take<S>(&mut self, spanned: S) -> Result<(), IrError>
    where
        S: Spanned,
    {
        if self.budget == 0 {
            return Err(IrError::new(spanned, IrErrorKind::BudgetExceeded));
        }

        self.budget -= 1;
        Ok(())
    }
}
