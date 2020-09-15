//! Simplified scope implementation used for indexing.

use crate::collections::{HashMap, HashSet};
use crate::{CompileError, CompileErrorKind};
use runestick::{CompileMetaCapture, Span};
use std::rc::Rc;
use std::{cell::RefCell, mem::ManuallyDrop};

#[derive(Debug)]
pub struct IndexScopeGuard {
    levels: Rc<RefCell<Vec<IndexScopeLevel>>>,
}

impl IndexScopeGuard {
    /// Pop the last closure scope and return captured variables.
    pub(crate) fn into_closure(self, span: Span) -> Result<Closure, CompileError> {
        let this = ManuallyDrop::new(self);

        let level = this
            .levels
            .borrow_mut()
            .pop()
            .ok_or_else(|| CompileError::internal(span, "missing scope"))?;

        match level {
            IndexScopeLevel::IndexClosure(closure) => Ok(Closure {
                captures: closure.captures,
                generator: closure.generator,
                is_async: closure.is_async,
                has_await: closure.has_await,
            }),
            _ => Err(CompileError::internal(span, "expected closure")),
        }
    }

    /// Pop the last function scope and return function information.
    pub(crate) fn into_function(self, span: Span) -> Result<Function, CompileError> {
        let this = ManuallyDrop::new(self);

        let level = this
            .levels
            .borrow_mut()
            .pop()
            .ok_or_else(|| CompileError::internal(span, "missing scope"))?;

        match level {
            IndexScopeLevel::IndexFunction(fun) => Ok(Function {
                generator: fun.generator,
                is_async: fun.is_async,
                has_await: fun.has_await,
            }),
            _ => Err(CompileError::internal(span, "expected function")),
        }
    }
}

impl Drop for IndexScopeGuard {
    fn drop(&mut self) {
        let exists = self.levels.borrow_mut().pop().is_some();
        debug_assert!(exists);
    }
}

#[derive(Debug, Clone)]
struct IndexScope {
    locals: HashMap<String, Span>,
}

impl IndexScope {
    /// Construct a new scope.
    pub fn new() -> Self {
        Self {
            locals: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexClosure {
    is_async: bool,
    /// Variables which could not be found in the immediate scope, and
    /// marked as needed to be captured from the outer scope.
    captures: Vec<CompileMetaCapture>,
    existing: HashSet<String>,
    scope: IndexScope,
    generator: bool,
    has_await: bool,
}

impl IndexClosure {
    /// Construct a new closure.
    pub fn new(is_async: bool) -> Self {
        Self {
            is_async,
            captures: Vec::new(),
            existing: HashSet::new(),
            scope: IndexScope::new(),
            generator: false,
            has_await: false,
        }
    }
}

pub(crate) struct Function {
    pub(crate) generator: bool,
    pub(crate) is_async: bool,
    #[allow(dead_code)]
    pub(crate) has_await: bool,
}

pub(crate) struct Closure {
    pub(crate) captures: Vec<CompileMetaCapture>,
    pub(crate) generator: bool,
    pub(crate) is_async: bool,
    #[allow(dead_code)]
    pub(crate) has_await: bool,
}

#[derive(Debug, Clone)]
pub struct IndexFunction {
    is_async: bool,
    scope: IndexScope,
    generator: bool,
    has_await: bool,
}

impl IndexFunction {
    /// Construct a new function.
    pub fn new(is_async: bool) -> Self {
        Self {
            is_async,
            scope: IndexScope::new(),
            generator: false,
            has_await: false,
        }
    }
}

#[derive(Debug, Clone)]
enum IndexScopeLevel {
    /// A regular index scope.
    IndexScope(IndexScope),
    /// A marker for a closure boundary.
    ///
    /// The scope is the first scope inside of the closure.
    IndexClosure(IndexClosure),
    /// A function (completely isolated scope-wise).
    IndexFunction(IndexFunction),
}

/// An indexing scope.
#[derive(Debug)]
pub struct IndexScopes {
    levels: Rc<RefCell<Vec<IndexScopeLevel>>>,
}

impl IndexScopes {
    /// Construct a new handler for indexing scopes.
    pub fn new() -> Self {
        Self {
            levels: Rc::new(RefCell::new(vec![IndexScopeLevel::IndexScope(
                IndexScope::new(),
            )])),
        }
    }

    /// Take a snapshot of the current scopes.
    pub fn snapshot(&self) -> Self {
        Self {
            levels: Rc::new(RefCell::new(self.levels.borrow().clone())),
        }
    }

    /// Declare the given variable in the last scope.
    pub fn declare(&mut self, var: &str, span: Span) -> Result<(), CompileError> {
        let mut levels = self.levels.borrow_mut();

        let level = levels
            .last_mut()
            .ok_or_else(|| CompileError::internal(span, "empty scopes"))?;

        let scope = match level {
            IndexScopeLevel::IndexScope(scope) => scope,
            IndexScopeLevel::IndexClosure(closure) => &mut closure.scope,
            IndexScopeLevel::IndexFunction(fun) => &mut fun.scope,
        };

        scope.locals.insert(var.to_owned(), span);
        Ok(())
    }

    /// Mark that the given variable is used.
    pub fn mark_use(&mut self, var: &str) {
        let mut levels = self.levels.borrow_mut();
        let iter = levels.iter_mut().rev();

        let mut closures = Vec::new();
        let mut found = false;

        for level in iter {
            match level {
                IndexScopeLevel::IndexScope(scope) => {
                    if scope.locals.get(var).is_some() {
                        found = true;
                        break;
                    }
                }
                IndexScopeLevel::IndexClosure(closure) => {
                    if closure.existing.contains(var) {
                        found = true;
                        break;
                    }

                    if closure.scope.locals.get(var).is_some() {
                        found = true;
                        break;
                    }

                    closures.push(closure);
                }
                // NB: cannot capture variables outside of functions.
                IndexScopeLevel::IndexFunction(scope) => {
                    found = scope.scope.locals.get(var).is_some();
                    break;
                }
            }
        }

        // mark all traversed closures to capture the given variable.
        if found {
            for closure in closures {
                closure.captures.push(CompileMetaCapture {
                    ident: var.to_owned(),
                });

                let inserted = closure.existing.insert(var.to_owned());

                // NB: should be checked above, because closures where it's
                // already captured are skipped.
                debug_assert!(inserted);
            }
        }
    }

    /// Mark that a yield was used, meaning the encapsulating function is a
    /// generator.
    pub fn mark_yield(&mut self, span: Span) -> Result<(), CompileError> {
        let mut levels = self.levels.borrow_mut();
        let iter = levels.iter_mut().rev();

        for level in iter {
            match level {
                IndexScopeLevel::IndexFunction(fun) => {
                    fun.generator = true;
                    return Ok(());
                }
                IndexScopeLevel::IndexClosure(closure) => {
                    closure.generator = true;
                    return Ok(());
                }
                IndexScopeLevel::IndexScope(..) => (),
            }
        }

        Err(CompileError::new(
            span,
            CompileErrorKind::YieldOutsideFunction,
        ))
    }

    /// Mark that a yield was used, meaning the encapsulating function is a
    /// generator.
    pub fn mark_await(&mut self, span: Span) -> Result<(), CompileError> {
        let mut levels = self.levels.borrow_mut();
        let iter = levels.iter_mut().rev();

        for level in iter {
            match level {
                IndexScopeLevel::IndexFunction(fun) => {
                    if fun.is_async {
                        fun.has_await = true;
                        return Ok(());
                    }

                    break;
                }
                IndexScopeLevel::IndexClosure(closure) => {
                    if closure.is_async {
                        closure.has_await = true;
                        return Ok(());
                    }

                    break;
                }
                IndexScopeLevel::IndexScope(..) => (),
            }
        }

        Err(CompileError::new(
            span,
            CompileErrorKind::AwaitOutsideFunction,
        ))
    }

    /// Push a function.
    pub fn push_function(&mut self, is_async: bool) -> IndexScopeGuard {
        self.levels
            .borrow_mut()
            .push(IndexScopeLevel::IndexFunction(IndexFunction::new(is_async)));

        IndexScopeGuard {
            levels: self.levels.clone(),
        }
    }

    /// Push a closure boundary.
    pub fn push_closure(&mut self, is_async: bool) -> IndexScopeGuard {
        self.levels
            .borrow_mut()
            .push(IndexScopeLevel::IndexClosure(IndexClosure::new(is_async)));

        IndexScopeGuard {
            levels: self.levels.clone(),
        }
    }

    /// Push a new scope.
    pub fn push_scope(&mut self) -> IndexScopeGuard {
        self.levels
            .borrow_mut()
            .push(IndexScopeLevel::IndexScope(IndexScope::new()));

        IndexScopeGuard {
            levels: self.levels.clone(),
        }
    }
}
