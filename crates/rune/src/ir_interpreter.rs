use crate::collections::HashMap;
use crate::eval::{Eval as _, Used};
use crate::query::Query;
use crate::{CompileError, CompileErrorKind, Spanned};
use runestick::{CompileMetaKind, ConstValue, Item, Span};
use std::cell::RefCell;
use std::rc::Rc;

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
        ir: &rune_ir::Ir,
        used: Used,
    ) -> Result<ConstValue, CompileError> {
        log::trace!("processing constant: {}", self.item);

        if let Some(const_value) = self.query.consts.get(&self.item) {
            return Ok(const_value);
        }

        if !self.query.consts.mark(&self.item) {
            return Err(CompileError::new(ir, CompileErrorKind::ConstCycle));
        }

        let const_value = match self.eval(ir, used) {
            Ok(const_value) => const_value,
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
    ) -> Result<ConstValue, CompileError> {
        if let Some(const_value) = self.scopes.get(ident) {
            return Ok(const_value);
        }

        let mut base = self.item.clone();

        while !base.is_empty() {
            base.pop();
            let item = base.extended(ident);

            if let Some(const_value) = self.query.consts.get(&item) {
                return Ok(const_value);
            }

            let meta = match self.query.query_meta_with_use(&item, used)? {
                Some(meta) => meta,
                None => continue,
            };

            match &meta.kind {
                CompileMetaKind::Const { const_value, .. } => return Ok(const_value.clone()),
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

pub(crate) struct IrScopeGuard {
    length: usize,
    scopes: Rc<RefCell<Vec<IrScope>>>,
}

impl Drop for IrScopeGuard {
    fn drop(&mut self) {
        // Note on panic: it shouldn't be possible for this to panic except for
        // grievous internal errors. Scope guards can only be created in
        // `IrScopes::push`, and it guarantees that there are a certain
        // number of scopes.
        let mut scopes = self.scopes.borrow_mut();
        assert!(scopes.pop().is_some(), "expected at least one scope");

        if scopes.len() != self.length {
            panic!("scope length mismatch");
        }
    }
}

#[derive(Default)]
pub(crate) struct IrScope {
    /// Locals in the current scope.
    locals: HashMap<String, ConstValue>,
}

/// A hierarchy of constant scopes.
pub(crate) struct IrScopes {
    scopes: Rc<RefCell<Vec<IrScope>>>,
}

impl IrScopes {
    /// Get a value out of the scope.
    pub(crate) fn get(&self, name: &str) -> Option<ConstValue> {
        let scopes = self.scopes.borrow();

        for scope in scopes.iter().rev() {
            if let Some(current) = scope.locals.get(name) {
                return Some(current.clone());
            }
        }

        None
    }

    /// Declare a value in the scope.
    pub(crate) fn decl<S>(
        &self,
        name: &str,
        value: ConstValue,
        spanned: S,
    ) -> Result<(), CompileError>
    where
        S: Spanned,
    {
        let mut scopes = self.scopes.borrow_mut();
        let last = scopes
            .last_mut()
            .ok_or_else(|| CompileError::internal(spanned, "expected at least one scope"))?;
        last.locals.insert(name.to_owned(), value);
        Ok(())
    }

    /// Replace the value of a variable in the scope
    ///
    /// The variable must have been declared in a scope beforehand.
    pub(crate) fn replace<S>(
        &self,
        name: &str,
        value: ConstValue,
        spanned: S,
    ) -> Result<(), CompileError>
    where
        S: Spanned,
    {
        let mut scopes = self.scopes.borrow_mut();

        for scope in scopes.iter_mut().rev() {
            if let Some(current) = scope.locals.get_mut(name) {
                *current = value;
                return Ok(());
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
    pub(crate) fn push(&self) -> IrScopeGuard {
        let length = {
            let mut scopes = self.scopes.borrow_mut();
            let length = scopes.len();
            scopes.push(IrScope::default());
            length
        };

        IrScopeGuard {
            length,
            scopes: self.scopes.clone(),
        }
    }
}

impl Default for IrScopes {
    fn default() -> Self {
        Self {
            scopes: Rc::new(RefCell::new(vec![IrScope::default()])),
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
