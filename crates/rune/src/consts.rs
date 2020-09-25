//! Constants storage.
//!
//! This maps the item of a global constant to its value. It's also used to
//! detect resolution cycles during constant evaluation.

use crate::collections::{HashMap, HashSet};
use runestick::{ConstValue, Item};
use std::cell::RefCell;
use std::rc::Rc;

/// State for constants processing.
#[derive(Default)]
pub(crate) struct Consts {
    inner: Rc<RefCell<Inner>>,
}

impl Consts {
    /// Mark that the given constant is being processed.
    ///
    /// Returns `true` if the given constant hasn't been marked yet. This is
    /// used to detect cycles during processing.
    pub(crate) fn mark(&self, item: &Item) -> bool {
        let mut inner = self.inner.borrow_mut();
        inner.processing.insert(item.clone())
    }

    /// Get the value for the constant at the given item, if present.
    pub(crate) fn get(&self, item: &Item) -> Option<ConstValue> {
        self.inner.borrow().resolved.get(item).cloned()
    }

    /// Insert a constant value at the given item.
    pub(crate) fn insert(&self, item: Item, value: ConstValue) -> Option<ConstValue> {
        self.inner.borrow_mut().resolved.insert(item, value)
    }
}

#[derive(Default)]
struct Inner {
    /// Const expression that have been resolved.
    resolved: HashMap<Item, ConstValue>,
    /// Constant expressions being processed.
    processing: HashSet<Item>,
}
