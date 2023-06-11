#![allow(unused)]

use core::cell::RefCell;
use core::fmt;
use core::num::NonZeroUsize;

use crate::no_std::collections::{BTreeSet, HashSet};
use crate::no_std::prelude::*;
use crate::no_std::vec::Vec;

use crate::compile::error::{MissingScope, PopError};

use rune_macros::instrument;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct Scope(usize);

/// The kind of a layer.
#[derive(Default)]
enum LayerKind {
    #[default]
    Default,
    Captures,
}

/// An owned capture.
#[derive(Debug, Clone)]
pub(crate) enum OwnedName {
    SelfValue,
    Str(String),
    Id(usize),
}

impl fmt::Display for OwnedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwnedName::SelfValue => "self".fmt(f),
            OwnedName::Str(name) => name.fmt(f),
            OwnedName::Id(id) => id.fmt(f),
        }
    }
}

/// A captured variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Name<'hir> {
    /// Capture of the `self` value.
    SelfValue,
    /// Capture of a named variable.
    Str(&'hir str),
    /// Anonymous variable.
    Id(usize),
}

impl<'hir> Name<'hir> {
    /// Coerce into an owned name.
    pub(crate) fn into_owned(self) -> OwnedName {
        match self {
            Name::SelfValue => OwnedName::SelfValue,
            Name::Str(name) => OwnedName::Str(name.to_owned()),
            Name::Id(id) => OwnedName::Id(id),
        }
    }
}

impl fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Name::SelfValue => "self".fmt(f),
            Name::Str(name) => name.fmt(f),
            Name::Id(id) => id.fmt(f),
        }
    }
}

#[derive(Default)]
pub(crate) struct Layer<'hir> {
    /// Scope identifier of the layer.
    scope: Scope,
    /// The parent layer.
    parent: Option<NonZeroUsize>,
    ///  The kind of the layer.
    kind: LayerKind,
    /// Variables defined in this layer.
    variables: HashSet<Name<'hir>>,
    /// Order of variable definitions.
    order: Vec<Name<'hir>>,
    /// Captures inside of this layer.
    captures: BTreeSet<Name<'hir>>,
}

impl<'hir> Layer<'hir> {
    fn parent(&self) -> Option<usize> {
        Some(self.parent?.get().wrapping_sub(1))
    }

    /// Convert layer into variable drop order.
    #[inline(always)]
    pub(crate) fn into_drop_order(self) -> impl ExactSizeIterator<Item = Name<'hir>> {
        self.order.into_iter().rev()
    }

    /// Variables captured by the layer.
    pub(crate) fn captures(&self) -> impl ExactSizeIterator<Item = Name<'hir>> + '_ {
        self.captures.iter().copied()
    }
}

pub(crate) struct Scopes<'hir> {
    scope: Scope,
    scopes: slab::Slab<Layer<'hir>>,
    ids: usize,
}

impl<'hir> Scopes<'hir> {
    /// Root scope.
    pub const ROOT: Scope = Scope(0);

    /// Push a scope.
    pub(crate) fn push(&mut self) {
        let scope = Scope(self.scopes.vacant_key());

        let layer = Layer {
            scope,
            parent: Some(NonZeroUsize::new(self.scope.0.wrapping_add(1)).expect("ran out of ids")),
            variables: HashSet::new(),
            order: Vec::new(),
            kind: LayerKind::Default,
            captures: BTreeSet::new(),
        };

        self.scopes.insert(layer);
        self.scope = scope;
    }

    /// Push an async block.
    pub(crate) fn push_captures(&mut self) {
        let scope = Scope(self.scopes.vacant_key());

        let layer = Layer {
            scope,
            parent: Some(NonZeroUsize::new(self.scope.0.wrapping_add(1)).expect("ran out of ids")),
            variables: HashSet::new(),
            order: Vec::new(),
            kind: LayerKind::Captures,
            captures: BTreeSet::new(),
        };

        self.scopes.insert(layer);
        self.scope = scope;
    }

    /// Pop the given scope.
    #[tracing::instrument(skip_all, fields(?self.scope))]
    pub(crate) fn pop(&mut self) -> Result<Layer<'hir>, PopError> {
        let Some(layer) = self.scopes.try_remove(self.scope.0) else {
            return Err(PopError::MissingScope(self.scope.0));
        };

        let Some(parent) = layer.parent() else {
            return Err(PopError::MissingParentScope(self.scope.0));
        };

        let to = Scope(parent);
        tracing::trace!(from = ?self.scope, ?to);
        self.scope = to;
        Ok(layer)
    }

    /// Define the given variable.
    #[tracing::instrument(skip_all, fields(?self.scope, ?name))]
    pub(crate) fn define(&mut self, name: Name<'hir>) -> Result<Name<'hir>, MissingScope> {
        tracing::trace!(?self.scope, ?name, "define");

        let Some(layer) = self.scopes.get_mut(self.scope.0) else {
            return Err(MissingScope(self.scope.0));
        };

        layer.variables.insert(name);
        layer.order.push(name);
        Ok(name)
    }

    /// Try to lookup the given variable.
    #[tracing::instrument(skip_all, fields(?self.scope, ?name))]
    pub(crate) fn get(&mut self, name: Name<'hir>) -> Option<(Name<'hir>, Scope)> {
        tracing::trace!("get");

        let mut blocks = Vec::new();
        let mut scope = self.scopes.get(self.scope.0);

        let scope = 'ok: {
            while let Some(layer) = scope.take() {
                if layer.variables.contains(&name) {
                    break 'ok layer.scope;
                }

                if let LayerKind::Captures { .. } = layer.kind {
                    blocks.push(layer.scope);
                }

                tracing::trace!(parent = ?layer.parent());
                scope = self.scopes.get(layer.parent()?);
            }

            return None;
        };

        for s in blocks {
            let Some(layer) = self.scopes.get_mut(s.0) else {
                continue;
            };

            layer.captures.insert(name);
        }

        Some((name, scope))
    }
}

impl<'hir> Default for Scopes<'hir> {
    #[inline]
    fn default() -> Self {
        let mut scopes = slab::Slab::new();
        scopes.insert(Layer::default());

        Self {
            scope: Scopes::ROOT,
            scopes,
            ids: 0,
        }
    }
}
