//! Simplified scope implementation used for indexing.

use crate::ast::Span;
use crate::collections::{HashMap, HashSet};
use crate::compile::{CaptureMeta, CompileError, CompileErrorKind};
use std::cell::RefCell;
use std::rc::Rc;

/// The kind of an indexed function.
#[derive(Debug, Clone, Copy)]
pub(crate) enum IndexFnKind {
    None,
    Const,
    Async,
}

#[derive(Debug)]
#[must_use]
pub struct IndexScopeGuard {
    id: usize,
    levels: Rc<RefCell<Vec<IndexScopeLevel>>>,
    consumed: bool,
}

impl IndexScopeGuard {
    /// Pop the last closure scope and return captured variables.
    pub(crate) fn into_closure(mut self, span: Span) -> Result<Closure, CompileError> {
        self.consumed = true;

        let level = self
            .levels
            .borrow_mut()
            .pop()
            .ok_or_else(|| CompileError::msg(&span, "missing scope"))?;

        debug_assert_eq!(level.scope().id, self.id);

        match level {
            IndexScopeLevel::IndexClosure(closure) if self.id == closure.scope.id => Ok(Closure {
                kind: closure.kind,
                do_move: closure.do_move,
                captures: closure.captures,
                generator: closure.generator,
                has_await: closure.has_await,
            }),
            _ => Err(CompileError::msg(&span, "expected closure")),
        }
    }

    /// Pop the last function scope and return function information.
    pub(crate) fn into_function(mut self, span: Span) -> Result<Function, CompileError> {
        self.consumed = true;

        let level = self
            .levels
            .borrow_mut()
            .pop()
            .ok_or_else(|| CompileError::msg(&span, "missing function"))?;

        debug_assert_eq!(level.scope().id, self.id);

        match level {
            IndexScopeLevel::IndexFunction(fun) => Ok(Function {
                generator: fun.generator,
                kind: fun.kind,
                has_await: fun.has_await,
            }),
            _ => Err(CompileError::msg(&span, "expected function")),
        }
    }
}

impl Drop for IndexScopeGuard {
    fn drop(&mut self) {
        if !self.consumed {
            let removed = self.levels.borrow_mut().pop();
            let level = removed.expect("expected scope level");
            assert_eq!(level.scope().id, self.id);
        }
    }
}

#[derive(Debug, Clone)]
struct IndexScope {
    /// Unique identifier assigned to every scope to ensure that it matches the
    /// hierarchy at each point where the scope guard is consumed so that we
    /// can correctly detect programming bugs.
    id: usize,
    locals: HashMap<String, Span>,
}

impl IndexScope {
    /// Construct a new scope.
    pub fn new(id: usize) -> Self {
        Self {
            id,
            locals: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct IndexClosure {
    kind: IndexFnKind,
    /// Perform a move.
    do_move: bool,
    /// Variables which could not be found in the immediate scope, and
    /// marked as needed to be captured from the outer scope.
    captures: Vec<CaptureMeta>,
    existing: HashSet<String>,
    scope: IndexScope,
    generator: bool,
    has_await: bool,
}

impl IndexClosure {
    /// Construct a new closure.
    pub(crate) fn new(id: usize, kind: IndexFnKind, do_move: bool) -> Self {
        Self {
            kind,
            do_move,
            captures: Vec::new(),
            existing: HashSet::new(),
            scope: IndexScope::new(id),
            generator: false,
            has_await: false,
        }
    }
}

pub(crate) struct Function {
    pub(crate) generator: bool,
    pub(crate) kind: IndexFnKind,
    #[allow(dead_code)]
    pub(crate) has_await: bool,
}

pub(crate) struct Closure {
    pub(crate) kind: IndexFnKind,
    pub(crate) do_move: bool,
    pub(crate) captures: Vec<CaptureMeta>,
    pub(crate) generator: bool,
    #[allow(dead_code)]
    pub(crate) has_await: bool,
}

#[derive(Debug, Clone)]
pub struct IndexFunction {
    kind: IndexFnKind,
    scope: IndexScope,
    generator: bool,
    has_await: bool,
}

impl IndexFunction {
    /// Construct a new function.
    pub(crate) fn new(index: usize, kind: IndexFnKind) -> Self {
        Self {
            kind,
            scope: IndexScope::new(index),
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

impl IndexScopeLevel {
    fn scope(&self) -> &IndexScope {
        match self {
            IndexScopeLevel::IndexScope(scope) => scope,
            IndexScopeLevel::IndexClosure(closure) => &closure.scope,
            IndexScopeLevel::IndexFunction(fun) => &fun.scope,
        }
    }
}

/// An indexing scope.
#[derive(Debug)]
pub(crate) struct IndexScopes {
    id: usize,
    levels: Rc<RefCell<Vec<IndexScopeLevel>>>,
}

impl IndexScopes {
    /// Construct a new handler for indexing scopes.
    pub(crate) fn new() -> Self {
        Self {
            id: 1,
            levels: Rc::new(RefCell::new(vec![IndexScopeLevel::IndexScope(
                IndexScope::new(0),
            )])),
        }
    }

    /// Declare the given variable in the last scope.
    pub(crate) fn declare(&mut self, var: &str, span: Span) -> Result<(), CompileError> {
        let mut levels = self.levels.borrow_mut();

        let level = levels
            .last_mut()
            .ok_or_else(|| CompileError::msg(&span, "empty scopes"))?;

        let scope = match level {
            IndexScopeLevel::IndexScope(scope) => scope,
            IndexScopeLevel::IndexClosure(closure) => &mut closure.scope,
            IndexScopeLevel::IndexFunction(fun) => &mut fun.scope,
        };

        scope.locals.insert(var.to_owned(), span);
        Ok(())
    }

    /// Mark that the given variable is used.
    pub(crate) fn mark_use(&mut self, var: &str) {
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
                closure.captures.push(CaptureMeta { ident: var.into() });

                let inserted = closure.existing.insert(var.into());

                // NB: should be checked above, because closures where it's
                // already captured are skipped.
                debug_assert!(inserted);
            }
        }
    }

    /// Mark that a yield was used, meaning the encapsulating function is a
    /// generator.
    pub(crate) fn mark_yield(&mut self, span: Span) -> Result<(), CompileError> {
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
    pub(crate) fn mark_await(&mut self, span: Span) -> Result<(), CompileError> {
        let mut levels = self.levels.borrow_mut();
        let iter = levels.iter_mut().rev();

        for level in iter {
            match level {
                IndexScopeLevel::IndexFunction(fun) => {
                    if let IndexFnKind::Async = fun.kind {
                        fun.has_await = true;
                        return Ok(());
                    }

                    break;
                }
                IndexScopeLevel::IndexClosure(closure) => {
                    if let IndexFnKind::Async = closure.kind {
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
    pub(crate) fn push_function(&mut self, kind: IndexFnKind) -> IndexScopeGuard {
        let id = self.id();
        let mut levels = self.levels.borrow_mut();
        levels.push(IndexScopeLevel::IndexFunction(IndexFunction::new(id, kind)));

        IndexScopeGuard {
            id,
            levels: self.levels.clone(),
            consumed: false,
        }
    }

    /// Push a closure boundary.
    pub(crate) fn push_closure(&mut self, kind: IndexFnKind, do_move: bool) -> IndexScopeGuard {
        let id = self.id();
        let mut levels = self.levels.borrow_mut();
        levels.push(IndexScopeLevel::IndexClosure(IndexClosure::new(
            id, kind, do_move,
        )));

        IndexScopeGuard {
            id,
            levels: self.levels.clone(),
            consumed: false,
        }
    }

    /// Push a new scope.
    pub(crate) fn push_scope(&mut self) -> IndexScopeGuard {
        let id = self.id();
        let mut levels = self.levels.borrow_mut();
        levels.push(IndexScopeLevel::IndexScope(IndexScope::new(id)));

        IndexScopeGuard {
            id,
            levels: self.levels.clone(),
            consumed: false,
        }
    }

    /// Allocate the next scope id.
    fn id(&mut self) -> usize {
        let next = self.id;
        self.id += 1;
        next
    }
}
