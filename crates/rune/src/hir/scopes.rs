#![allow(unused)]

use core::cell::RefCell;
use core::fmt;
use core::num::NonZeroUsize;

use crate::alloc::prelude::*;
use crate::alloc::{self, BTreeSet, HashMap, Vec};
use crate::ast::Spanned;
use crate::compile::error::{MissingScope, PopError};
use crate::compile::{self, HasSpan};
use crate::hir;
use crate::parse::NonZeroId;
use crate::shared::Gen;

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
    variables: HashMap<hir::Name<'hir>, hir::Variable>,
    /// Order of variable definitions.
    order: Vec<hir::Variable>,
    /// Captures inside of this layer.
    captures: BTreeSet<(hir::Name<'hir>, hir::Variable)>,
    /// An optional layer label.
    label: Option<&'hir str>,
}

impl<'hir> Layer<'hir> {
    fn parent(&self) -> Option<usize> {
        Some(self.parent?.get().wrapping_sub(1))
    }

    /// Convert layer into variable drop order.
    #[inline(always)]
    pub(crate) fn into_drop_order(self) -> impl ExactSizeIterator<Item = hir::Variable> {
        self.order.into_iter().rev()
    }

    /// Variables captured by the layer.
    pub(crate) fn captures(
        &self,
    ) -> impl ExactSizeIterator<Item = (hir::Name<'hir>, hir::Variable)> + '_ {
        self.captures.iter().copied()
    }
}

pub(crate) struct Scopes<'hir, 'a> {
    scope: Scope,
    scopes: Vec<Layer<'hir>>,
    gen: &'a Gen,
}

impl<'hir, 'a> Scopes<'hir, 'a> {
    /// Root scope.
    pub const ROOT: Scope = Scope(0);

    #[inline]
    pub(crate) fn new(gen: &'a Gen) -> alloc::Result<Self> {
        let mut scopes = Vec::new();
        scopes.try_push(Layer::default())?;

        Ok(Self {
            scope: Scopes::ROOT,
            scopes,
            gen,
        })
    }

    /// Push a scope.
    pub(crate) fn push(&mut self, label: Option<&'hir str>) -> alloc::Result<()> {
        self.push_kind(LayerKind::Default, label)
    }

    /// Push an async block.
    pub(crate) fn push_captures(&mut self) -> alloc::Result<()> {
        self.push_kind(LayerKind::Captures, None)
    }

    /// Push a loop.
    pub(crate) fn push_loop(&mut self, label: Option<&'hir str>) -> alloc::Result<()> {
        self.push_kind(LayerKind::Loop, label)
    }

    fn push_kind(&mut self, kind: LayerKind, label: Option<&'hir str>) -> alloc::Result<()> {
        let scope = Scope(self.scopes.len());

        let layer = Layer {
            scope,
            parent: Some(NonZeroUsize::new(self.scope.0.wrapping_add(1)).expect("ran out of ids")),
            variables: HashMap::new(),
            order: Vec::new(),
            kind,
            captures: BTreeSet::new(),
            label,
        };

        self.scopes.try_push(layer)?;
        self.scope = scope;
        Ok(())
    }

    /// Pop the given scope.
    #[tracing::instrument(skip_all, fields(?self.scope))]
    pub(crate) fn pop(&mut self) -> Result<Layer<'hir>, PopError> {
        let Some(layer) = self.scopes.pop() else {
            return Err(PopError::MissingScope(self.scope.0));
        };

        if layer.scope.0 != self.scope.0 {
            return Err(PopError::MissingScope(self.scope.0));
        }

        let Some(parent) = layer.parent() else {
            return Err(PopError::MissingParentScope(self.scope.0));
        };

        let to = Scope(parent);
        tracing::trace!(from = ?self.scope, ?to);
        self.scope = to;
        Ok(layer)
    }

    /// Insert a variable with the specified id.
    #[tracing::instrument(skip_all, fields(?self.scope, ?name))]
    pub(crate) fn insert(
        &mut self,
        name: hir::Name<'hir>,
        id: hir::Variable,
        span: &dyn Spanned,
    ) -> compile::Result<hir::Variable> {
        tracing::trace!(?self.scope, ?name, "define");

        let Some(layer) = self.scopes.get_mut(self.scope.0) else {
            return Err(HasSpan::new(span, MissingScope(self.scope.0)).into());
        };

        layer.variables.try_insert(name, id)?;
        layer.order.try_push(id)?;
        Ok(id)
    }

    /// Define the given variable.
    #[tracing::instrument(skip_all, fields(?self.scope, ?name))]
    pub(crate) fn define(
        &mut self,
        name: hir::Name<'hir>,
        span: &dyn Spanned,
    ) -> compile::Result<hir::Variable> {
        tracing::trace!(?self.scope, ?name, "define");

        let Some(layer) = self.scopes.get_mut(self.scope.0) else {
            return Err(HasSpan::new(span, MissingScope(self.scope.0)).into());
        };

        let id = hir::Variable(self.gen.next());
        layer.variables.try_insert(name, id)?;
        layer.order.try_push(id)?;
        Ok(id)
    }

    /// Try to lookup the given variable.
    #[tracing::instrument(skip_all, fields(?self.scope, ?name))]
    pub(crate) fn get(
        &mut self,
        name: hir::Name<'hir>,
    ) -> alloc::Result<Option<(hir::Variable, Scope)>> {
        tracing::trace!("get");

        let mut blocks = Vec::new();
        let mut scope = self.scopes.get(self.scope.0);

        let (scope, id) = 'ok: {
            loop {
                let Some(layer) = scope.take() else {
                    return Ok(None);
                };

                if let Some(id) = layer.variables.get(&name) {
                    break 'ok (layer.scope, *id);
                }

                if let LayerKind::Captures { .. } = layer.kind {
                    blocks.try_push(layer.scope)?;
                }

                tracing::trace!(parent = ?layer.parent());

                let Some(parent) = layer.parent() else {
                    return Ok(None);
                };

                scope = self.scopes.get(parent);
            }
        };

        for s in blocks {
            let Some(layer) = self.scopes.get_mut(s.0) else {
                continue;
            };

            layer.captures.try_insert((name, id))?;
        }

        Ok(Some((id, scope)))
    }

    /// Walk the loop and construct captures for it.
    #[tracing::instrument(skip_all, fields(?self.scope, ?label))]
    pub(crate) fn loop_drop(
        &self,
        label: Option<&str>,
    ) -> alloc::Result<Option<Vec<hir::Variable>>> {
        let mut captures = Vec::new();
        let mut scope = self.scopes.get(self.scope.0);

        while let Some(layer) = scope.take() {
            if let Some(label) = label {
                if layer.label == Some(label) {
                    return Ok(Some(captures));
                }
            } else if matches!(layer.kind, LayerKind::Loop) {
                return Ok(Some(captures));
            }

            captures.try_extend(layer.order.iter().rev().copied())?;
            tracing::trace!(parent = ?layer.parent());

            let Some(parent) = layer.parent() else {
                return Ok(None);
            };

            scope = self.scopes.get(parent);
        }

        Ok(None)
    }
}
