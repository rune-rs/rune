use crate::collections::HashMap;
use crate::compiler::{CompileError, Result};
use runestick::unit::Span;

/// A locally declared variable.
#[derive(Debug, Clone)]
pub(super) struct Var {
    /// Slot offset from the current stack frame.
    pub(super) offset: usize,
    /// Name of the variable.
    name: String,
    /// Token assocaited with the variable.
    span: Span,
    /// Flag indicating that the variable has been moved.
    pub(super) moved: Option<Span>,
}

/// A locally declared variable.
#[derive(Debug, Clone)]
pub(super) struct AnonVar {
    /// Slot offset from the current stack frame.
    offset: usize,
    /// Span associated with the anonymous variable.
    span: Span,
}

#[derive(Debug, Clone)]
pub(super) struct Scope {
    /// Named variables.
    locals: HashMap<String, Var>,
    /// Anonymous variables.
    anon: Vec<AnonVar>,
    /// The number of variables.
    pub(super) total_var_count: usize,
    /// The number of variables local to this scope.
    pub(super) local_var_count: usize,
}

impl Scope {
    /// Construct a new locals handlers.
    pub(super) fn new() -> Scope {
        Self {
            locals: HashMap::new(),
            anon: Vec::new(),
            total_var_count: 0,
            local_var_count: 0,
        }
    }

    /// Construct a new child scope.
    pub(super) fn child(&self) -> Self {
        Self {
            locals: HashMap::new(),
            anon: Vec::new(),
            total_var_count: self.total_var_count,
            local_var_count: 0,
        }
    }

    /// Insert a new local, and return the old one if there's a conflict.
    pub(super) fn new_var(&mut self, name: &str, span: Span) -> Result<()> {
        let local = Var {
            offset: self.total_var_count,
            name: name.to_owned(),
            span,
            moved: None,
        };

        self.total_var_count += 1;
        self.local_var_count += 1;

        if let Some(old) = self.locals.insert(name.to_owned(), local) {
            return Err(CompileError::VariableConflict {
                name: name.to_owned(),
                span,
                existing_span: old.span,
            });
        }

        Ok(())
    }

    /// Insert a new local, and return the old one if there's a conflict.
    pub(super) fn decl_var(&mut self, name: &str, span: Span) -> usize {
        let offset = self.total_var_count;

        log::trace!("decl {} => {}", name, offset);

        self.locals.insert(
            name.to_owned(),
            Var {
                offset,
                name: name.to_owned(),
                span,
                moved: None,
            },
        );

        self.total_var_count += 1;
        self.local_var_count += 1;
        offset
    }

    /// Insert a new local, and return the old one if there's a conflict.
    pub(super) fn decl_anon(&mut self, span: Span) -> usize {
        let offset = self.total_var_count;

        self.anon.push(AnonVar { offset, span });

        self.total_var_count += 1;
        self.local_var_count += 1;
        offset
    }

    /// Access the variable with the given name.
    pub(super) fn get(&self, name: &str) -> Option<&Var> {
        if let Some(var) = self.locals.get(name) {
            return Some(var);
        }

        None
    }

    /// Access the variable mutably with the given name.
    pub(super) fn get_mut(&mut self, name: &str) -> Option<&mut Var> {
        if let Some(var) = self.locals.get_mut(name) {
            return Some(var);
        }

        None
    }
}

/// A guard returned from [push][Scopes::push].
///
/// This should be provided to a subsequent [pop][Scopes::pop] to allow it to be
/// sanity checked.
#[must_use]
pub(super) struct ScopeGuard(usize);

pub(super) struct Scopes {
    scopes: Vec<Scope>,
}

impl Scopes {
    /// Construct a new collection of scopes.
    pub(super) fn new() -> Self {
        Self {
            scopes: vec![Scope::new()],
        }
    }

    /// Try to get the local with the given name. Returns `None` if it's
    /// missing.
    pub(super) fn try_get_var(&self, name: &str, span: Span) -> Result<Option<&Var>> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                if let Some(moved_at) = var.moved {
                    return Err(CompileError::MovedLocal {
                        name: name.to_owned(),
                        span,
                        moved_at,
                    });
                }

                return Ok(Some(var));
            }
        }

        Ok(None)
    }

    /// Get the local with the given name.
    pub(super) fn get_var(&self, name: &str, span: Span) -> Result<&Var> {
        match self.try_get_var(name, span)? {
            Some(var) => Ok(var),
            None => Err(CompileError::MissingLocal {
                name: name.to_owned(),
                span,
            }),
        }
    }

    /// Try to get the local mutably with the given name. Returns `None` if it's
    /// missing.
    pub(super) fn try_move_var(&mut self, name: &str, span: Span) -> Result<Option<&Var>> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(name) {
                if let Some(moved_at) = var.moved {
                    return Err(CompileError::MovedLocal {
                        name: name.to_owned(),
                        span,
                        moved_at,
                    });
                }

                var.moved = Some(span);
                return Ok(Some(var));
            }
        }

        Ok(None)
    }

    /// Get the local with the given name.
    pub(super) fn last(&self, span: Span) -> Result<&Scope> {
        Ok(self
            .scopes
            .last()
            .ok_or_else(|| CompileError::internal("missing head of locals", span))?)
    }

    /// Get the last locals scope.
    pub(super) fn last_mut(&mut self, span: Span) -> Result<&mut Scope> {
        Ok(self
            .scopes
            .last_mut()
            .ok_or_else(|| CompileError::internal("missing head of locals", span))?)
    }

    /// Push a scope and return an index.
    pub(super) fn push(&mut self, scope: Scope) -> ScopeGuard {
        self.scopes.push(scope);
        ScopeGuard(self.scopes.len())
    }

    /// Pop the last scope and compare with the expected length.
    pub(super) fn pop(&mut self, span: Span, expected: ScopeGuard) -> Result<Scope> {
        let ScopeGuard(expected) = expected;

        if self.scopes.len() != expected {
            return Err(CompileError::internal(
                "the number of scopes do not match",
                span,
            ));
        }

        self.pop_unchecked(span)
    }

    /// Pop the last of the scope.
    pub(super) fn pop_last(&mut self, span: Span) -> Result<Scope> {
        self.pop(span, ScopeGuard(1))
    }

    /// Pop the last scope and compare with the expected length.
    pub(super) fn pop_unchecked(&mut self, span: Span) -> Result<Scope> {
        let scope = self
            .scopes
            .pop()
            .ok_or_else(|| CompileError::internal("missing parent scope", span))?;

        Ok(scope)
    }
}
