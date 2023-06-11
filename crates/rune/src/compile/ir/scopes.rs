use crate::no_std::collections::HashMap;
use crate::no_std::prelude::*;

use crate::compile::ir;
use crate::hir;

/// Error indicating that a local variable is missing.
pub(crate) struct MissingLocal(pub(crate) hir::OwnedName);

/// A hierarchy of constant scopes.
pub(crate) struct Scopes {
    scopes: Vec<Scope>,
}

impl Scopes {
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
    pub(crate) fn decl(
        &mut self,
        name: &hir::OwnedName,
        value: ir::Value,
    ) -> Result<(), &'static str> {
        let last = self.last_mut().ok_or("expected at least one scope")?;
        last.locals.insert(name.clone(), value);
        Ok(())
    }

    /// Try to get the value out from the scopes.
    pub(crate) fn try_get(&self, name: &hir::OwnedName) -> Option<&ir::Value> {
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
    pub(crate) fn get_name(&self, name: &hir::OwnedName) -> Result<&ir::Value, MissingLocal> {
        for scope in self.scopes.iter().rev() {
            if let Some(current) = scope.locals.get(name) {
                return Ok(current);
            }

            // don't look past isolate scopes.
            if let ScopeKind::Isolate = scope.kind {
                break;
            }
        }

        Err(MissingLocal(name.clone()))
    }

    /// Get the given variable as mutable.
    pub(crate) fn get_name_mut(
        &mut self,
        name: &hir::OwnedName,
    ) -> Result<&mut ir::Value, MissingLocal> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(current) = scope.locals.get_mut(name) {
                return Ok(current);
            }

            // don't look past isolate scopes.
            if let ScopeKind::Isolate = scope.kind {
                break;
            }
        }

        Err(MissingLocal(name.clone()))
    }

    /// Push a scope and return the guard associated with the scope.
    pub(crate) fn push(&mut self) -> ScopeGuard {
        let length = self.scopes.len();
        self.scopes.push(Scope::default());
        ScopeGuard { length }
    }

    /// Push an isolate scope and return the guard associated with the scope.
    pub(crate) fn isolate(&mut self) -> ScopeGuard {
        let length = self.scopes.len();
        let scope = Scope {
            kind: ScopeKind::Isolate,
            ..Default::default()
        };
        self.scopes.push(scope);
        ScopeGuard { length }
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

impl Default for Scopes {
    fn default() -> Self {
        Self {
            scopes: vec![Scope::default()],
        }
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
    locals: HashMap<hir::OwnedName, ir::Value>,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            kind: ScopeKind::None,
            locals: Default::default(),
        }
    }
}
