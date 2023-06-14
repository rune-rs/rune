#![allow(unused)]

use core::cell::RefCell;
use core::fmt;
use core::num::NonZeroUsize;

use crate::no_std::collections::{BTreeSet, HashSet};
use crate::no_std::prelude::*;
use crate::no_std::vec::Vec;

use crate::compile::error::{MissingScope, PopError};
use crate::hir;

use rune_macros::instrument;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct Scope(usize);

/// The kind of a layer.
#[derive(Default)]
enum LayerKind {
    #[default]
    Default,
    Loop,
    Captures,
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
    variables: HashSet<hir::Name<'hir>>,
    /// Order of variable definitions.
    order: Vec<hir::Name<'hir>>,
    /// Captures inside of this layer.
    captures: BTreeSet<hir::Name<'hir>>,
    /// An optional layer label.
    label: Option<&'hir str>,
}

impl<'hir> Layer<'hir> {
    fn parent(&self) -> Option<usize> {
        Some(self.parent?.get().wrapping_sub(1))
    }

    /// Convert layer into variable drop order.
    #[inline(always)]
    pub(crate) fn into_drop_order(self) -> impl ExactSizeIterator<Item = hir::Name<'hir>> {
        self.order.into_iter().rev()
    }

    /// Variables captured by the layer.
    pub(crate) fn captures(&self) -> impl ExactSizeIterator<Item = hir::Name<'hir>> + '_ {
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
        self.push_kind(LayerKind::Default, None)
    }

    /// Push an async block.
    pub(crate) fn push_captures(&mut self) {
        self.push_kind(LayerKind::Captures, None)
    }

    /// Push a loop.
    pub(crate) fn push_loop(&mut self, label: Option<&'hir str>) {
        self.push_kind(LayerKind::Loop, label)
    }

    fn push_kind(&mut self, kind: LayerKind, label: Option<&'hir str>) {
        let scope = Scope(self.scopes.vacant_key());

        let layer = Layer {
            scope,
            parent: Some(NonZeroUsize::new(self.scope.0.wrapping_add(1)).expect("ran out of ids")),
            variables: HashSet::new(),
            order: Vec::new(),
            kind,
            captures: BTreeSet::new(),
            label,
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
    pub(crate) fn define(
        &mut self,
        name: hir::Name<'hir>,
    ) -> Result<hir::Name<'hir>, MissingScope> {
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
    pub(crate) fn get(&mut self, name: hir::Name<'hir>) -> Option<(hir::Name<'hir>, Scope)> {
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

    /// Walk the loop and construct captures for it.
    #[tracing::instrument(skip_all, fields(?self.scope, ?label))]
    pub(crate) fn loop_drop(&self, label: Option<&str>) -> Option<Vec<hir::Name<'hir>>> {
        let mut captures = Vec::new();
        let mut scope = self.scopes.get(self.scope.0);

        while let Some(layer) = scope.take() {
            if let Some(label) = label {
                if layer.label == Some(label) {
                    return Some(captures);
                }
            } else if matches!(layer.kind, LayerKind::Loop) {
                return Some(captures);
            }

            captures.extend(layer.order.iter().rev().copied());
            tracing::trace!(parent = ?layer.parent());
            scope = self.scopes.get(layer.parent()?);
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
            scope: Scopes::ROOT,
            scopes,
            ids: 0,
        }
    }
}
