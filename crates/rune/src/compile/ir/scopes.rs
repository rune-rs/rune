use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap};
use crate::ast::Spanned;
use crate::compile::{self, ErrorKind};
use crate::hir;
use crate::runtime::Value;

/// Error indicating that a local variable is missing.
pub(crate) struct MissingLocal(pub(crate) Box<str>);

/// A hierarchy of constant scopes.
pub(crate) struct Scopes {
    scopes: Vec<Scope>,
}

impl Scopes {
    /// Construct a new empty scope.
    pub(crate) fn new() -> alloc::Result<Self> {
        Ok(Self {
            scopes: try_vec![Scope::default()],
        })
    }

    /// Clear the current scope.
    pub(crate) fn clear_current(&mut self) -> Result<(), &'static str> {
        let last = self
            .scopes
            .last_mut()
            .ok_or("expected at least one scope")?;

        last.locals.clear();
        Ok(())
    }

    /// Declare a value in the scope.
    pub(crate) fn decl(&mut self, name: hir::Variable, value: Value) -> Result<(), ErrorKind> {
        let last = self
            .last_mut()
            .ok_or_else(|| ErrorKind::msg("Expected at least one scope"))?;
        last.locals.try_insert(name, value)?;
        Ok(())
    }

    /// Try to get the value out from the scopes.
    pub(crate) fn try_get(&self, name: &hir::Variable) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(current) = scope.locals.get(name) {
                return Some(current);
            }

            // don't look past isolate scopes.
            if let ScopeKind::Isolate = scope.kind {
                break;
            }
        }

        None
    }

    /// Get the given variable.
    pub(crate) fn get_name(
        &self,
        name: &hir::Variable,
        span: &dyn Spanned,
    ) -> compile::Result<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(current) = scope.locals.get(name) {
                return Ok(current);
            }

            // don't look past isolate scopes.
            if let ScopeKind::Isolate = scope.kind {
                break;
            }
        }

        Err(compile::Error::new(
            span,
            MissingLocal(name.try_to_string()?.try_into_boxed_str()?),
        ))
    }

    /// Get the given variable as mutable.
    pub(crate) fn get_name_mut(
        &mut self,
        name: &hir::Variable,
        span: &dyn Spanned,
    ) -> compile::Result<&mut Value> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(current) = scope.locals.get_mut(name) {
                return Ok(current);
            }

            // don't look past isolate scopes.
            if let ScopeKind::Isolate = scope.kind {
                break;
            }
        }

        Err(compile::Error::new(
            span,
            MissingLocal(name.try_to_string()?.try_into_boxed_str()?),
        ))
    }

    /// Push a scope and return the guard associated with the scope.
    pub(crate) fn push(&mut self) -> alloc::Result<ScopeGuard> {
        let length = self.scopes.len();
        self.scopes.try_push(Scope::default())?;
        Ok(ScopeGuard { length })
    }

    /// Push an isolate scope and return the guard associated with the scope.
    pub(crate) fn isolate(&mut self) -> alloc::Result<ScopeGuard> {
        let length = self.scopes.len();
        let scope = Scope {
            kind: ScopeKind::Isolate,
            ..Default::default()
        };
        self.scopes.try_push(scope)?;
        Ok(ScopeGuard { length })
    }

    pub(crate) fn pop(&mut self, guard: ScopeGuard) -> Result<(), &'static str> {
        if self.scopes.pop().is_none() {
            return Err("expected at least one scope to pop");
        }

        if self.scopes.len() != guard.length {
            return Err("scope length mismatch");
        }

        Ok(())
    }

    /// Get the last scope mutably.
    pub(crate) fn last_mut(&mut self) -> Option<&mut Scope> {
        self.scopes.last_mut()
    }
}

#[repr(transparent)]
pub(crate) struct ScopeGuard {
    length: usize,
}

#[derive(Debug, Clone, Copy)]
enum ScopeKind {
    None,
    Isolate,
}

pub(crate) struct Scope {
    kind: ScopeKind,
    /// Locals in the current scope.
    locals: HashMap<hir::Variable, Value>,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            kind: ScopeKind::None,
            locals: HashMap::new(),
        }
    }
}
