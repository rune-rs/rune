#![allow(unused)]

use core::cell::RefCell;
use core::fmt;
use core::num::NonZeroUsize;

use crate::no_std::collections::BTreeSet;
use crate::no_std::collections::HashMap;
use crate::no_std::vec::Vec;

use crate::ast::Span;
use crate::compile::error::{MissingScope, PopError};

use rune_macros::instrument;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct Scope(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub(crate) struct Variable(usize);

/// The kind of a layer.
#[derive(Default)]
enum LayerKind {
    #[default]
    Default,
    Captures,
}

#[derive(Default)]
pub(crate) struct Layer {
    scope: Scope,
    parent: Option<NonZeroUsize>,
    pub(crate) awaits: Vec<Span>,
    pub(crate) yields: Vec<Span>,
}

impl Layer {
    fn parent(&self) -> Option<usize> {
        Some(self.parent?.get().wrapping_sub(1))
    }
}

pub(crate) struct Scopes {
    scope: Scope,
    scopes: slab::Slab<Layer>,
}

impl Scopes {
    /// Root scope.
    pub const ROOT: Scope = Scope(0);

    /// Push a scope.
    pub(crate) fn push(&mut self) {
        let scope = Scope(self.scopes.vacant_key());

        let layer = Layer {
            scope,
            parent: Some(NonZeroUsize::new(self.scope.0.wrapping_add(1)).expect("ran out of ids")),
            awaits: Vec::new(),
            yields: Vec::new(),
        };

        self.scopes.insert(layer);
        self.scope = scope;
    }

    /// Pop the given scope.
    pub(crate) fn pop(&mut self) -> Result<Layer, PopError> {
        let Some(layer) = self.scopes.try_remove(self.scope.0) else {
            return Err(PopError::MissingScope(self.scope.0));
        };

        let Some(parent) = layer.parent() else {
            return Err(PopError::MissingParentScope(self.scope.0));
        };

        self.scope = Scope(parent);
        Ok(layer)
    }

    /// Define the given variable.
    #[tracing::instrument(skip_all)]
    pub(crate) fn mark<F>(&mut self, f: F) -> Result<(), MissingScope>
    where
        F: FnOnce(&mut Layer),
    {
        tracing::trace!(?self.scope, "mark await");

        let Some(layer) = self.scopes.get_mut(self.scope.0) else {
            return Err(MissingScope(self.scope.0));
        };

        f(layer);
        Ok(())
    }
}

impl Default for Scopes {
    #[inline]
    fn default() -> Self {
        let mut scopes = slab::Slab::new();
        scopes.insert(Layer::default());

        Self {
            scope: Scopes::ROOT,
            scopes,
        }
    }
}
