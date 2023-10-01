#![allow(unused)]

use core::cell::RefCell;
use core::fmt;
use core::num::NonZeroUsize;

use crate::alloc::{self, BTreeSet, HashMap, Vec};
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
    scopes: Vec<Layer>,
}

impl Scopes {
    /// Root scope.
    pub const ROOT: Scope = Scope(0);

    pub(crate) fn new() -> alloc::Result<Self> {
        let mut scopes = Vec::new();
        scopes.try_push(Layer::default())?;

        Ok(Self {
            scope: Scopes::ROOT,
            scopes,
        })
    }

    /// Push a scope.
    pub(crate) fn push(&mut self) -> alloc::Result<()> {
        let scope = Scope(self.scopes.len());

        let layer = Layer {
            scope,
            parent: Some(NonZeroUsize::new(self.scope.0.wrapping_add(1)).expect("ran out of ids")),
            awaits: Vec::new(),
            yields: Vec::new(),
        };

        self.scopes.try_push(layer)?;
        self.scope = scope;
        Ok(())
    }

    /// Pop the given scope.
    pub(crate) fn pop(&mut self) -> Result<Layer, PopError> {
        let Some(layer) = self.scopes.pop() else {
            return Err(PopError::MissingScope(self.scope.0));
        };

        if layer.scope.0 != self.scope.0 {
            return Err(PopError::MissingScope(self.scope.0));
        }

        let Some(parent) = layer.parent() else {
            return Err(PopError::MissingParentScope(self.scope.0));
        };

        self.scope = Scope(parent);
        Ok(layer)
    }

    /// Define the given variable.
    #[tracing::instrument(skip_all)]
    pub(crate) fn mark(&mut self) -> Result<&mut Layer, MissingScope> {
        tracing::trace!(?self.scope, "mark await");

        let Some(layer) = self.scopes.get_mut(self.scope.0) else {
            return Err(MissingScope(self.scope.0));
        };

        Ok(layer)
    }
}
