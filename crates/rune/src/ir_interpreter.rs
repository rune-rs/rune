use crate::collections::HashMap;
use crate::eval::{Eval as _, Used};
use crate::ir;
use crate::ir_value::IrValue;
use crate::query::Query;
use crate::{CompileError, CompileErrorKind, Spanned};
use runestick::{CompileMetaKind, ConstValue, Item, Span};

/// The compiler phase which evaluates constants.
pub(crate) struct IrInterpreter<'a> {
    /// A budget associated with the compiler, for how many expressions it's
    /// allowed to evaluate.
    pub(crate) budget: IrBudget,
    /// The item where the constant expression is located.
    pub(crate) item: Item,
    /// Query engine to look for constant expressions.
    pub(crate) query: &'a mut Query,
    /// Constant scopes.
    pub(crate) scopes: IrScopes,
}

impl<'a> IrInterpreter<'a> {
    /// Outer evaluation for an expression which performs caching into `consts`.
    pub(crate) fn eval_expr(
        &mut self,
        ir: &ir::Ir,
        used: Used,
    ) -> Result<ConstValue, CompileError> {
        log::trace!("processing constant: {}", self.item);

        if let Some(const_value) = self.query.consts.get(&self.item) {
            return Ok(const_value);
        }

        if !self.query.consts.mark(&self.item) {
            return Err(CompileError::new(ir, CompileErrorKind::ConstCycle));
        }

        let ir_value = match self.eval(ir, used) {
            Ok(ir_value) => ir_value,
            Err(outcome) => match outcome {
                crate::eval::EvalOutcome::Error(error) => {
                    return Err(error);
                }
                crate::eval::EvalOutcome::NotConst(span) => {
                    return Err(CompileError::new(span, CompileErrorKind::NotConst))
                }
                crate::eval::EvalOutcome::Break(span, _) => {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::BreakOutsideOfLoop,
                    ))
                }
            },
        };

        let const_value = ir_value.into_const(ir)?;

        if self
            .query
            .consts
            .insert(self.item.clone(), const_value.clone())
            .is_some()
        {
            return Err(CompileError::new(ir, CompileErrorKind::ConstCycle));
        }

        Ok(const_value)
    }

    /// Resolve the given constant value from the block scope.
    ///
    /// This looks up `const <ident> = <expr>` and evaluates them while caching
    /// their result.
    pub(crate) fn resolve_var(
        &mut self,
        ident: &str,
        span: Span,
        used: Used,
    ) -> Result<IrValue, CompileError> {
        if let Some(const_value) = self.scopes.get(ident) {
            return Ok(const_value);
        }

        let mut base = self.item.clone();

        while !base.is_empty() {
            base.pop();
            let item = base.extended(ident);

            if let Some(const_value) = self.query.consts.get(&item) {
                return Ok(IrValue::from_const(const_value));
            }

            let meta = match self.query.query_meta_with_use(&item, used)? {
                Some(meta) => meta,
                None => continue,
            };

            match &meta.kind {
                CompileMetaKind::Const { const_value, .. } => {
                    return Ok(IrValue::from_const(const_value.clone()));
                }
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

#[repr(transparent)]
pub(crate) struct IrScopeGuard {
    length: usize,
}

#[derive(Default)]
pub(crate) struct IrScope {
    /// Locals in the current scope.
    locals: HashMap<String, IrValue>,
}

/// A hierarchy of constant scopes.
pub(crate) struct IrScopes {
    scopes: Vec<IrScope>,
}

impl IrScopes {
    /// Get a value out of the scope.
    pub(crate) fn get(&self, name: &str) -> Option<IrValue> {
        for scope in self.scopes.iter().rev() {
            if let Some(current) = scope.locals.get(name) {
                return Some(current.clone());
            }
        }

        None
    }

    /// Declare a value in the scope.
    pub(crate) fn decl<S>(
        &mut self,
        name: &str,
        value: IrValue,
        spanned: S,
    ) -> Result<(), CompileError>
    where
        S: Spanned,
    {
        let last = self
            .scopes
            .last_mut()
            .ok_or_else(|| CompileError::internal(spanned, "expected at least one scope"))?;
        last.locals.insert(name.to_owned(), value);
        Ok(())
    }

    /// Get the given variable as mutable.
    pub(crate) fn get_name<S>(&self, name: &str, spanned: S) -> Result<IrValue, CompileError>
    where
        S: Spanned,
    {
        for scope in self.scopes.iter().rev() {
            if let Some(current) = scope.locals.get(name) {
                return Ok(current.clone());
            }
        }

        Err(CompileError::new(
            spanned,
            CompileErrorKind::MissingLocal {
                name: name.to_owned(),
            },
        ))
    }

    /// Push a scope and return the guard associated with the scope.
    pub(crate) fn push(&mut self) -> IrScopeGuard {
        let length = self.scopes.len();
        self.scopes.push(IrScope::default());
        IrScopeGuard { length }
    }

    pub(crate) fn pop<S>(&mut self, spanned: S, guard: IrScopeGuard) -> Result<(), CompileError>
    where
        S: Spanned,
    {
        if self.scopes.pop().is_none() {
            return Err(CompileError::const_error(
                spanned,
                "expected at least one scope to pop",
            ));
        }

        if self.scopes.len() != guard.length {
            return Err(CompileError::const_error(spanned, "scope length mismatch"));
        }

        Ok(())
    }

    /// Get the given target as mut.
    pub(crate) fn get_target(&mut self, ir_target: &ir::IrTarget) -> Result<IrValue, CompileError> {
        match &ir_target.kind {
            ir::IrTargetKind::Name(name) => {
                return self.get_name(name, ir_target);
            }
            ir::IrTargetKind::Field(ir_target, field) => {
                let value = self.get_target(ir_target)?;

                match value {
                    IrValue::Object(object) => {
                        let object = object
                            .borrow_ref()
                            .map_err(|e| CompileError::access(ir_target, e))?;

                        if let Some(value) = object.get(field.as_ref()).cloned() {
                            return Ok(value);
                        }
                    }
                    actual => {
                        return Err(CompileError::const_expected::<_, runestick::Tuple>(
                            ir_target, &actual,
                        ))
                    }
                };

                Err(CompileError::const_error(ir_target, "missing field"))
            }
            ir::IrTargetKind::Index(target, index) => {
                let value = self.get_target(target)?;

                match value {
                    IrValue::Vec(vec) => {
                        let vec = vec
                            .borrow_ref()
                            .map_err(|e| CompileError::access(ir_target, e))?;

                        if let Some(value) = vec.get(*index).cloned() {
                            return Ok(value);
                        }
                    }
                    IrValue::Tuple(tuple) => {
                        let tuple = tuple
                            .borrow_ref()
                            .map_err(|e| CompileError::access(ir_target, e))?;

                        if let Some(value) = tuple.get(*index).cloned() {
                            return Ok(value);
                        }
                    }
                    actual => {
                        return Err(CompileError::const_expected::<_, runestick::Tuple>(
                            ir_target, &actual,
                        ))
                    }
                };

                Err(CompileError::const_error(ir_target, "missing index"))
            }
        }
    }

    /// Update the given target with the given constant value.
    pub(crate) fn set_target(
        &mut self,
        ir_target: &ir::IrTarget,
        value: IrValue,
    ) -> Result<(), CompileError> {
        match &ir_target.kind {
            ir::IrTargetKind::Name(name) => {
                let scope = self
                    .scopes
                    .last_mut()
                    .ok_or_else(|| CompileError::const_error(ir_target, "no scopes"))?;

                scope.locals.insert(name.as_ref().to_owned(), value);
                Ok(())
            }
            ir::IrTargetKind::Field(target, field) => {
                let current = self.get_target(target)?;

                match current {
                    IrValue::Object(object) => {
                        let mut object = object
                            .borrow_mut()
                            .map_err(|e| CompileError::access(ir_target, e))?;
                        object.insert(field.as_ref().to_owned(), value);
                    }
                    actual => {
                        return Err(CompileError::const_expected::<_, runestick::Object>(
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
                        let mut vec = vec
                            .borrow_mut()
                            .map_err(|e| CompileError::access(ir_target, e))?;

                        if let Some(current) = vec.get_mut(*index) {
                            *current = value;
                            return Ok(());
                        }
                    }
                    IrValue::Tuple(tuple) => {
                        let mut tuple = tuple
                            .borrow_mut()
                            .map_err(|e| CompileError::access(ir_target, e))?;

                        if let Some(current) = tuple.get_mut(*index) {
                            *current = value;
                            return Ok(());
                        }
                    }
                    actual => {
                        return Err(CompileError::const_expected::<_, runestick::Tuple>(
                            ir_target, &actual,
                        ));
                    }
                };

                Err(CompileError::const_error(ir_target, "missing index"))
            }
        }
    }
}

impl Default for IrScopes {
    fn default() -> Self {
        Self {
            scopes: vec![IrScope::default()],
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
    pub(crate) fn take<S>(&mut self, spanned: S) -> Result<(), CompileError>
    where
        S: Spanned,
    {
        if self.budget == 0 {
            return Err(CompileError::const_error(
                spanned,
                "constant evaluation budget exceeded",
            ));
        }

        self.budget -= 1;
        Ok(())
    }
}
