use crate::collections::HashMap;
use crate::shared::Custom;
use crate::Spanned;

use thiserror::Error;

/// A hierarchy of constant scopes.
pub(crate) struct Scopes<T> {
    scopes: Vec<Scope<T>>,
}

impl<T> Scopes<T> {
    /// Clear the current scope.
    pub(crate) fn clear_current<S>(&mut self, spanned: S) -> Result<(), Custom>
    where
        S: Spanned,
    {
        let last = self
            .scopes
            .last_mut()
            .ok_or_else(|| Custom::new(spanned, "expected at least one scope"))?;

        last.locals.clear();
        Ok(())
    }

    /// Declare a value in the scope.
    pub(crate) fn decl<S>(&mut self, name: &str, value: T, spanned: S) -> Result<(), Custom>
    where
        S: Spanned,
    {
        let last = self
            .last_mut()
            .ok_or_else(|| Custom::new(spanned, "expected at least one scope"))?;

        last.locals.insert(name.to_owned(), value);
        Ok(())
    }

    /// Try to get the value out from the scopes.
    pub(crate) fn try_get<'a>(&'a self, name: &str) -> Option<&'a T> {
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
    pub(crate) fn get_name<'a, S>(&'a self, name: &str, spanned: S) -> Result<&'a T, ScopeError>
    where
        S: Spanned,
    {
        for scope in self.scopes.iter().rev() {
            if let Some(current) = scope.locals.get(name) {
                return Ok(current);
            }

            // don't look past isolate scopes.
            if let ScopeKind::Isolate = scope.kind {
                break;
            }
        }

        Err(ScopeError::new(
            spanned,
            ScopeErrorKind::MissingLocal { name: name.into() },
        ))
    }

    /// Get the given variable as mutable.
    pub(crate) fn get_name_mut<'a, S>(
        &'a mut self,
        name: &str,
        spanned: S,
    ) -> Result<&'a mut T, ScopeError>
    where
        S: Spanned,
    {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(current) = scope.locals.get_mut(name) {
                return Ok(current);
            }

            // don't look past isolate scopes.
            if let ScopeKind::Isolate = scope.kind {
                break;
            }
        }

        Err(ScopeError::new(
            spanned,
            ScopeErrorKind::MissingLocal { name: name.into() },
        ))
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
        let scope = Scope::<T> {
            kind: ScopeKind::Isolate,
            ..Default::default()
        };
        self.scopes.push(scope);
        ScopeGuard { length }
    }

    pub(crate) fn pop<S>(&mut self, spanned: S, guard: ScopeGuard) -> Result<(), Custom>
    where
        S: Spanned,
    {
        if self.scopes.pop().is_none() {
            return Err(Custom::new(spanned, "expected at least one scope to pop"));
        }

        if self.scopes.len() != guard.length {
            return Err(Custom::new(spanned, "scope length mismatch"));
        }

        Ok(())
    }

    /// Get the last scope mutably.
    pub(crate) fn last_mut(&mut self) -> Option<&mut Scope<T>> {
        self.scopes.last_mut()
    }
}

impl<T> Default for Scopes<T> {
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

pub(crate) struct Scope<T> {
    kind: ScopeKind,
    /// Locals in the current scope.
    locals: HashMap<String, T>,
}

impl<T> Default for Scope<T> {
    fn default() -> Self {
        Self {
            kind: ScopeKind::None,
            locals: Default::default(),
        }
    }
}

error! {
    /// An error cause by issues with scoping.
    #[derive(Debug)]
    pub struct ScopeError {
        kind: ScopeErrorKind,
    }
}

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum ScopeErrorKind {
    #[error("{message}")]
    Custom { message: &'static str },
    #[error("missing local {name}")]
    MissingLocal { name: Box<str> },
}
