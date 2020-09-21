use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::eval::{Eval as _, Used};
use crate::query::Query;
use crate::Resolve;
use crate::{CompileError, CompileErrorKind, Spanned};
use runestick::{CompileMetaKind, ConstValue, Item, Source, Span};
use std::cell::RefCell;
use std::rc::Rc;

/// The compiler phase which evaluates constants.
pub(crate) struct ConstCompiler<'a> {
    /// A budget associated with the compiler, for how many expressions it's
    /// allowed to evaluate.
    pub(crate) budget: ConstBudget,
    /// The item where the constant expression is located.
    pub(crate) item: Item,
    /// Source file used in processing.
    pub(crate) source: &'a Source,
    /// Query engine to look for constant expressions.
    pub(crate) query: &'a mut Query,
    /// Constant scopes.
    pub(crate) scopes: ConstScopes,
}

impl<'a> ConstCompiler<'a> {
    /// Resolve the given resolvable value.
    pub(crate) fn resolve<T>(&self, value: &T) -> Result<T::Output, CompileError>
    where
        T: Resolve<'a>,
    {
        Ok(value.resolve(&self.query.storage, self.source)?)
    }

    /// Outer evaluation for an expression which performs caching into `consts`.
    pub(crate) fn eval_expr(
        &mut self,
        expr: &ast::Expr,
        used: Used,
    ) -> Result<ConstValue, CompileError> {
        log::trace!("processing constant: {}", self.item);

        if let Some(const_value) = self.query.consts.borrow().resolved.get(&self.item).cloned() {
            return Ok(const_value);
        }

        if !self
            .query
            .consts
            .borrow_mut()
            .processing
            .insert(self.item.clone())
        {
            return Err(CompileError::new(expr, CompileErrorKind::ConstCycle));
        }

        let const_value = match self.eval(expr, used)? {
            Some(const_value) => const_value,
            None => {
                return Err(CompileError::new(expr, CompileErrorKind::NotConst));
            }
        };

        if self
            .query
            .consts
            .borrow_mut()
            .resolved
            .insert(self.item.clone(), const_value.clone())
            .is_some()
        {
            return Err(CompileError::new(expr, CompileErrorKind::ConstCycle));
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

            if let Some(const_value) = self.query.consts.borrow().resolved.get(&item).cloned() {
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

/// State for constants processing.
#[derive(Default)]
pub(crate) struct Consts {
    /// Const expression that have been resolved.
    pub(crate) resolved: HashMap<Item, ConstValue>,
    /// Constant expressions being processed.
    pub(crate) processing: HashSet<Item>,
}

pub(crate) struct ConstScopeGuard {
    length: usize,
    scopes: Rc<RefCell<Vec<ConstScope>>>,
}

impl Drop for ConstScopeGuard {
    fn drop(&mut self) {
        // Note on panic: it shouldn't be possible for this to panic except for
        // grievous internal errors. Scope guards can only be created in
        // `ConstScopes::push`, and it guarantees that there are a certain
        // number of scopes.
        let mut scopes = self.scopes.borrow_mut();
        assert!(scopes.pop().is_some(), "expected at least one scope");

        if scopes.len() != self.length {
            panic!("scope length mismatch");
        }
    }
}

#[derive(Default)]
pub(crate) struct ConstScope {
    /// Locals in the current scope.
    locals: HashMap<String, ConstValue>,
}

/// A hierarchy of constant scopes.
pub(crate) struct ConstScopes {
    scopes: Rc<RefCell<Vec<ConstScope>>>,
}

impl ConstScopes {
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
    pub(crate) fn decl(
        &self,
        name: &str,
        value: ConstValue,
        span: Span,
    ) -> Result<(), CompileError> {
        let mut scopes = self.scopes.borrow_mut();
        let last = scopes
            .last_mut()
            .ok_or_else(|| CompileError::internal(span, "expected at least one scope"))?;
        last.locals.insert(name.to_owned(), value);
        Ok(())
    }

    /// Replace the value of a variable in the scope
    ///
    /// The variable must have been declared in a scope beforehand.
    pub(crate) fn replace(
        &self,
        name: &str,
        value: ConstValue,
        span: Span,
    ) -> Result<(), CompileError> {
        let mut scopes = self.scopes.borrow_mut();

        for scope in scopes.iter_mut().rev() {
            if let Some(current) = scope.locals.get_mut(name) {
                *current = value;
                return Ok(());
            }
        }

        Err(CompileError::new(
            span,
            CompileErrorKind::MissingLocal {
                name: name.to_owned(),
            },
        ))
    }

    /// Push a scope and return the guard associated with the scope.
    pub(crate) fn push(&self) -> ConstScopeGuard {
        let length = {
            let mut scopes = self.scopes.borrow_mut();
            let length = scopes.len();
            scopes.push(ConstScope::default());
            length
        };

        ConstScopeGuard {
            length,
            scopes: self.scopes.clone(),
        }
    }
}

impl Default for ConstScopes {
    fn default() -> Self {
        Self {
            scopes: Rc::new(RefCell::new(vec![ConstScope::default()])),
        }
    }
}

/// A budget dictating the number of evaluations the compiler is allowed to do.
pub(crate) struct ConstBudget {
    budget: usize,
}

impl ConstBudget {
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
