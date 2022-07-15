//! Constants storage.
//!
//! This maps the item of a global constant to its value. It's also used to
//! detect resolution cycles during constant evaluation.

use crate::collections::{HashMap, HashSet};
use crate::compile::{Item, ItemBuf};
use crate::runtime::ConstValue;

/// State for constants processing.
#[derive(Default)]
pub(crate) struct Consts {
    /// Const expression that have been resolved.
    resolved: HashMap<ItemBuf, ConstValue>,
    /// Constant expressions being processed.
    processing: HashSet<ItemBuf>,
}

impl Consts {
    /// Mark that the given constant is being processed.
    ///
    /// Returns `true` if the given constant hasn't been marked yet. This is
    /// used to detect cycles during processing.
    pub(crate) fn mark(&mut self, item: &Item) -> bool {
        self.processing.insert(item.to_owned())
    }

    /// Get the value for the constant at the given item, if present.
    pub(crate) fn get(&self, item: &Item) -> Option<&ConstValue> {
        self.resolved.get(item)
    }

    /// Insert a constant value at the given item.
    pub(crate) fn insert(&mut self, item: ItemBuf, value: ConstValue) -> Option<ConstValue> {
        self.resolved.insert(item, value)
    }
}
