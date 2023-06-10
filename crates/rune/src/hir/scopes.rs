#![allow(unused)]

use core::cell::RefCell;
use core::fmt;
use core::num::NonZeroUsize;

use crate::no_std::collections::BTreeSet;
use crate::no_std::collections::HashMap;
use crate::no_std::prelude::*;
use crate::no_std::vec::Vec;

use crate::compile::error::{MissingScope, PopError};

use rune_macros::instrument;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct Scope(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub(crate) struct Variable(pub(crate) usize);

impl fmt::Display for Variable {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The kind of a layer.
#[derive(Default)]
enum LayerKind {
    #[default]
    Default,
    Captures,
}

/// An owned capture.
#[derive(Debug, Clone)]
pub(crate) enum OwnedCapture {
    SelfValue,
    Name(String),
}

/// A captured variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Name<'hir> {
    /// Capture of the `self` value.
    SelfValue,
    /// Capture of a named variable.
    Str(&'hir str),
}

impl<'hir> Name<'hir> {
    pub(crate) fn as_str(self) -> &'hir str {
        match self {
            Name::SelfValue => "self",
            Name::Str(name) => name,
        }
    }

    /// Get the captured string.
    pub(crate) fn into_string(self) -> String {
        String::from(self.as_str())
    }
}

impl<'hir> fmt::Display for Name<'hir> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl<'hir> From<&'hir str> for Name<'hir> {
    #[inline]
    fn from(string: &'hir str) -> Self {
        Name::Str(string)
    }
}

#[derive(Default)]
pub(crate) struct Layer<'hir> {
    scope: Scope,
    parent: Option<NonZeroUsize>,
    /// Indicates if `self` is defined in this layer.
    has_self: Option<usize>,
    /// Variables defined in this layer.
    variables: HashMap<&'hir str, usize>,
    /// Order of variable definitions.
    order: Vec<usize>,
    kind: LayerKind,
    /// Captures inside of this layer.
    captures: BTreeSet<(Variable, Name<'hir>)>,
}

impl<'hir> Layer<'hir> {
    fn parent(&self) -> Option<usize> {
        Some(self.parent?.get().wrapping_sub(1))
    }

    /// Convert layer into variable drop order.
    #[inline(always)]
    pub(crate) fn into_drop_order(self) -> impl ExactSizeIterator<Item = Variable> {
        self.order.into_iter().rev().map(Variable)
    }

    /// Variables captured by the layer.
    pub(crate) fn captures(&self) -> impl ExactSizeIterator<Item = (Variable, Name<'hir>)> + '_ {
        self.captures.iter().copied()
    }
}

pub(crate) struct Scopes<'hir> {
    scope: Scope,
    scopes: slab::Slab<Layer<'hir>>,
    variables: slab::Slab<()>,
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
            has_self: None,
            variables: HashMap::new(),
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
            has_self: None,
            variables: HashMap::new(),
            order: Vec::new(),
            kind: LayerKind::Captures,
            captures: BTreeSet::new(),
        };

        self.scopes.insert(layer);
        self.scope = scope;
    }

    /// Pop the given scope.
    pub(crate) fn pop(&mut self) -> Result<Layer<'hir>, PopError> {
        let Some(layer) = self.scopes.try_remove(self.scope.0) else {
            return Err(PopError::MissingScope(self.scope.0));
        };

        let Some(parent) = layer.parent() else {
            return Err(PopError::MissingParentScope(self.scope.0));
        };

        for &variable in &layer.order {
            if self.variables.try_remove(variable).is_none() {
                return Err(PopError::MissingVariable(variable));
            }
        }

        self.scope = Scope(parent);
        Ok(layer)
    }

    /// Define `self` value.
    pub(crate) fn define_self(&mut self) -> Result<Variable, MissingScope> {
        tracing::trace!(?self.scope, "define self");

        let Some(layer) = self.scopes.get_mut(self.scope.0) else {
            return Err(MissingScope(self.scope.0));
        };

        let variable = self.variables.insert(());
        layer.has_self = Some(variable);
        layer.order.push(variable);
        Ok(Variable(variable))
    }

    /// Define the given variable.
    #[tracing::instrument(skip_all)]
    pub(crate) fn define(&mut self, name: &'hir str) -> Result<Variable, MissingScope> {
        tracing::trace!(?self.scope, ?name, "define");

        let Some(layer) = self.scopes.get_mut(self.scope.0) else {
            return Err(MissingScope(self.scope.0));
        };

        let variable = self.variables.insert(());
        // Intentionally ignore shadowing variable assignments, since shadowed
        // variables aren't dropped until the end of the scope anyways.
        layer.variables.insert(name, variable);
        layer.order.push(variable);
        Ok(Variable(variable))
    }

    /// Try to lookup the self variable.
    #[tracing::instrument(skip_all)]
    pub(crate) fn get_self(&mut self) -> Option<Variable> {
        tracing::trace!(?self.scope, "get self");
        self.scan(|layer| Some((Variable(layer.has_self?), Name::SelfValue)))
    }

    /// Try to lookup the given variable.
    #[tracing::instrument(skip_all)]
    pub(crate) fn get(&mut self, name: &'hir str) -> Option<Variable> {
        tracing::trace!(?self.scope, ?name, "get");
        self.scan(|layer| Some((Variable(*layer.variables.get(name)?), Name::Str(name))))
    }

    fn scan<F>(&mut self, mut f: F) -> Option<Variable>
    where
        F: FnMut(&Layer) -> Option<(Variable, Name<'hir>)>,
    {
        let mut blocks = Vec::new();
        let mut scope = self.scopes.get(self.scope.0);

        let (variable, capture) = 'ok: {
            while let Some(s) = scope.take() {
                if let Some((variable, capture)) = f(s) {
                    break 'ok (variable, capture);
                }

                if let LayerKind::Captures { .. } = s.kind {
                    blocks.push(s.scope);
                }

                tracing::trace!(parent = ?s.parent());
                scope = self.scopes.get(s.parent()?);
            }

            return None;
        };

        for s in blocks {
            let Some(s) = self.scopes.get_mut(s.0) else {
                continue;
            };

            s.captures.insert((variable, capture));
        }

        Some(variable)
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
            variables: slab::Slab::new(),
        }
    }
}
