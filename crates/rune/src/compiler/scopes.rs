use crate::collections::HashMap;
use crate::compiler::{CompileError, Result};
use runestick::unit::Assembly;
use runestick::{Inst, Span};

/// A locally declared variable.
#[derive(Debug, Clone)]
pub(super) struct Local {
    /// Slot offset from the current stack frame.
    pub(super) offset: usize,
    /// Name of the variable.
    name: String,
    /// Token assocaited with the variable.
    span: Span,
}

/// A variable captures from the environment.
#[derive(Debug, Clone)]
pub(super) struct Environ {
    /// Slot offset from the current stack frame.
    pub(super) offset: usize,
    /// The index in the environment the variable comes from.
    pub(super) index: usize,
    /// The span the environment variable was declared in.
    span: Span,
}

impl Environ {
    /// Copy the given variable.
    pub fn copy(&self, asm: &mut Assembly, span: Span) {
        asm.push(
            Inst::TupleIndexGetAt {
                offset: self.offset,
                index: self.index,
            },
            span,
        );
    }
}

/// A declared variable.
#[derive(Debug, Clone)]
pub(super) enum Var {
    /// A locally declared variable.
    Local(Local),
    /// A variable captured in the environment.
    Environ(Environ),
}

impl Var {
    /// Get the span of the variable.
    pub fn span(&self) -> Span {
        match self {
            Self::Local(local) => local.span,
            Self::Environ(environ) => environ.span,
        }
    }

    /// Copy the declared variable.
    pub fn copy(&self, asm: &mut Assembly, span: Span) {
        match self {
            Self::Local(local) => {
                asm.push(
                    Inst::Copy {
                        offset: local.offset,
                    },
                    span,
                );
            }
            Self::Environ(environ) => {
                environ.copy(asm, span);
            }
        }
    }
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
    pub(super) fn new_env_var(
        &mut self,
        name: &str,
        offset: usize,
        index: usize,
        span: Span,
    ) -> Result<()> {
        let local = Var::Environ(Environ {
            offset,
            index,
            span,
        });

        if let Some(old) = self.locals.insert(name.to_owned(), local) {
            return Err(CompileError::VariableConflict {
                name: name.to_owned(),
                span,
                existing_span: old.span(),
            });
        }

        Ok(())
    }

    /// Insert a new local, and return the old one if there's a conflict.
    pub(super) fn new_var(&mut self, name: &str, span: Span) -> Result<()> {
        let local = Var::Local(Local {
            offset: self.total_var_count,
            name: name.to_owned(),
            span,
        });

        self.total_var_count += 1;
        self.local_var_count += 1;

        if let Some(old) = self.locals.insert(name.to_owned(), local) {
            return Err(CompileError::VariableConflict {
                name: name.to_owned(),
                span,
                existing_span: old.span(),
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
            Var::Local(Local {
                offset,
                name: name.to_owned(),
                span,
            }),
        );

        self.total_var_count += 1;
        self.local_var_count += 1;
        offset
    }

    /// Declare an anonymous variable.
    ///
    /// This is used if cleanup is required in the middle of an expression.
    pub(super) fn decl_anon(&mut self, span: Span) -> usize {
        let offset = self.total_var_count;

        self.anon.push(AnonVar { offset, span });

        self.total_var_count += 1;
        self.local_var_count += 1;
        offset
    }

    /// Undeclare the last anonymous variable.
    pub(super) fn undecl_anon(&mut self, n: usize, span: Span) -> Result<(), CompileError> {
        for _ in 0..n {
            self.anon.pop();
        }

        self.total_var_count = self
            .total_var_count
            .checked_sub(n)
            .ok_or_else(|| CompileError::internal("totals out of bounds", span))?;

        self.local_var_count = self
            .local_var_count
            .checked_sub(n)
            .ok_or_else(|| CompileError::internal("locals out of bounds", span))?;

        Ok(())
    }

    /// Access the variable with the given name.
    pub(super) fn get(&self, name: &str) -> Option<&Var> {
        if let Some(var) = self.locals.get(name) {
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
    pub(super) fn try_get_var(&self, name: &str) -> Result<Option<&Var>> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Ok(Some(var));
            }
        }

        Ok(None)
    }

    /// Get the local with the given name.
    pub(super) fn get_var(&self, name: &str, span: Span) -> Result<&Var> {
        match self.try_get_var(name)? {
            Some(var) => Ok(var),
            None => Err(CompileError::MissingLocal {
                name: name.to_owned(),
                span,
            }),
        }
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
    pub(super) fn pop(&mut self, expected: ScopeGuard, span: Span) -> Result<Scope> {
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
        self.pop(ScopeGuard(1), span)
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
