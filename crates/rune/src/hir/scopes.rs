#![allow(unused)]

use core::cell::RefCell;
use core::fmt;
use core::num::NonZeroUsize;

use crate::no_std::collections::HashMap;
use crate::no_std::vec::Vec;

#[derive(Debug)]
pub struct MissingScope(usize);

impl fmt::Display for MissingScope {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Missing scope with id {}", self.0)
    }
}

impl crate::no_std::error::Error for MissingScope {}

#[derive(Debug)]
pub enum PopError {
    MissingScope(MissingScope),
    MissingVariable(usize),
}

impl fmt::Display for PopError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PopError::MissingScope(error) => error.fmt(f),
            PopError::MissingVariable(id) => write!(f, "Missing variable with id {id}"),
        }
    }
}

impl crate::no_std::error::Error for PopError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct Scope(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct Variable(usize);

#[derive(Default)]
pub(crate) struct Layer<'hir> {
    parent: Option<NonZeroUsize>,
    variables: HashMap<&'hir str, usize>,
    order: Vec<usize>,
}

impl<'hir> Layer<'hir> {
    /// Convert layer into variable drop order.
    #[inline(always)]
    pub(crate) fn into_drop_order(self) -> impl ExactSizeIterator<Item = Variable> {
        self.order.into_iter().rev().map(Variable)
    }
}

pub(crate) struct Scopes<'hir> {
    scopes: slab::Slab<Layer<'hir>>,
    variables: slab::Slab<()>,
}

impl<'hir> Scopes<'hir> {
    /// Root scope.
    pub const ROOT: Scope = Scope(0);

    /// Push a scope.
    pub(crate) fn push(&mut self, scope: Scope) -> Scope {
        let layer = Layer {
            parent: Some(NonZeroUsize::new(scope.0.wrapping_add(1)).expect("ran out of ids")),
            variables: HashMap::new(),
            order: Vec::new(),
        };

        Scope(self.scopes.insert(layer))
    }

    /// Pop the given scope.
    #[must_use]
    pub(crate) fn pop(&mut self, scope: Scope) -> Result<Layer<'hir>, PopError> {
        let Some(layer) = self.scopes.try_remove(scope.0) else {
            return Err(PopError::MissingScope(MissingScope(scope.0)));
        };

        for &variable in &layer.order {
            if self.variables.try_remove(variable).is_none() {
                return Err(PopError::MissingVariable(variable));
            }
        }

        Ok(layer)
    }

    /// Define the given variable.
    pub(crate) fn define(
        &mut self,
        scope: Scope,
        ident: &'hir str,
    ) -> Result<Variable, MissingScope> {
        let Some(layer) = self.scopes.get_mut(scope.0) else {
            return Err(MissingScope(scope.0));
        };

        let variable = self.variables.insert(());
        // Intentionally ignore shadowing variable assignments, since shadowed
        // variables aren't dropped until the end of the scope anyways.
        layer.variables.insert(ident, variable);
        layer.order.push(variable);
        Ok(Variable(variable))
    }

    /// Try to lookup the given variable.
    pub(crate) fn get(&self, scope: Scope, name: &'hir str) -> Option<Variable> {
        let mut scope = self.scopes.get(scope.0);

        while let Some(s) = scope.take() {
            if let Some(variable) = s.variables.get(name) {
                return Some(Variable(*variable));
            }

            scope = self.scopes.get(s.parent?.get().wrapping_sub(1));
        }

        None
    }
}

impl<'hir> Default for Scopes<'hir> {
    #[inline]
    fn default() -> Self {
        let mut scopes = slab::Slab::new();
        scopes.insert(Layer::default());

        Self {
            scopes,
            variables: slab::Slab::new(),
        }
    }
}
