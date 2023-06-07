use core::cell::RefCell;
use core::num::NonZeroUsize;

use crate::no_std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct Scope(usize);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct Variable(usize);

#[derive(Default)]
struct Layer<'hir> {
    parent: Option<NonZeroUsize>,
    variables: HashMap<&'hir str, usize>,
}

pub(crate) struct Scopes<'hir> {
    scopes: RefCell<slab::Slab<Layer<'hir>>>,
}

impl<'hir> Scopes<'hir> {
    /// Root scope.
    pub const ROOT: Scope = Scope(0);

    /// Push a scope.
    pub(crate) fn push(&self, scope: Scope) -> Scope {
        let layer = Layer {
            parent: Some(NonZeroUsize::new(scope.0.wrapping_add(1)).expect("ran out of ids")),
            variables: HashMap::new(),
        };

        Scope(self.scopes.borrow_mut().insert(layer))
    }

    /// Pop the given scope.
    pub(crate) fn pop(&self, scope: Scope) {
        let mut scopes = self.scopes.borrow_mut();
        let _ = scopes.try_remove(scope.0);
    }

    /// Try to lookup the given variable.
    pub(crate) fn get(&self, scope: Scope, name: &'hir str) -> Option<Variable> {
        let scopes = self.scopes.borrow();
        let mut scope = scopes.get(scope.0);

        while let Some(s) = scope.take() {
            if let Some(variable) = s.variables.get(name) {
                return Some(Variable(*variable));
            }

            scope = scopes.get(s.parent?.get().wrapping_sub(1));
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
            scopes: RefCell::new(scopes),
        }
    }
}
