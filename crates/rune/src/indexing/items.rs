use core::fmt;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::item::ComponentRef;
use crate::{Item, ItemBuf};

#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct MissingLastId;

impl fmt::Display for MissingLastId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Missing last inserted id into the items stack")
    }
}

impl core::error::Error for MissingLastId {}

#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct GuardMismatch {
    actual: usize,
    expected: usize,
}

impl fmt::Display for GuardMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Guard mismatch when popping items, {} (actual) != {} (expected)",
            self.actual, self.expected
        )
    }
}

impl core::error::Error for GuardMismatch {}

/// Guard returned.
#[derive(Debug)]
#[must_use]
pub(crate) struct Guard(usize);

/// Manage item paths.
#[derive(Debug)]
pub(crate) struct Items {
    item: ItemBuf,
}

impl Items {
    /// Construct a new items manager.
    pub(crate) fn new(item: &Item) -> alloc::Result<Self> {
        Ok(Self {
            item: item.try_to_owned()?,
        })
    }

    /// Get the item for the current state of the path.
    pub(crate) fn item(&self) -> &Item {
        &self.item
    }

    /// Push a component and return a guard to it.
    pub(super) fn push_id(&mut self, id: usize) -> alloc::Result<Guard> {
        self.item.push(ComponentRef::Id(id))?;
        Ok(Guard(self.item.as_bytes().len()))
    }

    /// Push a component and return a guard to it.
    pub(super) fn push_name(&mut self, name: &str) -> alloc::Result<Guard> {
        self.item.push(name)?;
        Ok(Guard(self.item.as_bytes().len()))
    }

    /// Pop the scope associated with the given guard.
    pub(super) fn pop(&mut self, Guard(expected): Guard) -> Result<(), GuardMismatch> {
        if self.item.as_bytes().len() != expected {
            return Err(GuardMismatch {
                actual: self.item.as_bytes().len(),
                expected,
            });
        }

        self.item.pop();
        Ok(())
    }
}
