//! Constants storage.
//!
//! This maps the item of a global constant to its value. It's also used to
//! detect resolution cycles during constant evaluation.

use crate::alloc::{self, HashMap, HashSet};
use crate::compile::ItemId;
use crate::runtime::{ConstValue, ConstValueBuf};

/// State for constants processing.
#[derive(Default)]
pub(crate) struct Consts {
    /// Const expression that have been resolved.
    resolved: HashMap<ItemId, ConstValueBuf>,
    /// Constant expressions being processed.
    processing: HashSet<ItemId>,
}

impl Consts {
    /// Mark that the given constant is being processed.
    ///
    /// Returns `true` if the given constant hasn't been marked yet. This is
    /// used to detect cycles during processing.
    pub(crate) fn mark(&mut self, item: ItemId) -> alloc::Result<bool> {
        self.processing.try_insert(item)
    }

    /// Get the value for the constant at the given item, if present.
    pub(crate) fn get(&self, item: ItemId) -> Option<&ConstValue> {
        Some(self.resolved.get(&item)?)
    }

    /// Remove the value for the constant at the given item.
    ///
    /// This is used for static items, which are const-evaluated to get their
    /// initializer but must not become visible as constants.
    pub(crate) fn remove(&mut self, item: ItemId) -> Option<ConstValueBuf> {
        self.resolved.remove(&item)
    }

    /// Insert a constant value at the given item.
    pub(crate) fn insert(
        &mut self,
        item: ItemId,
        value: ConstValueBuf,
    ) -> alloc::Result<Option<ConstValueBuf>> {
        self.resolved.try_insert(item, value)
    }
}
