use core::fmt;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::compile::ErrorKind;
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

#[cfg(feature = "std")]
impl std::error::Error for MissingLastId {}

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

#[cfg(feature = "std")]
impl std::error::Error for GuardMismatch {}

/// Guard returned.
#[must_use]
pub(crate) struct Guard(usize);

/// Manage item paths.
#[derive(Debug)]
pub(crate) struct Items {
    block_index: usize,
    item: ItemBuf,
}

impl Items {
    /// Construct a new items manager.
    pub(crate) fn new(item: &Item) -> alloc::Result<Self> {
        Ok(Self {
            block_index: item.last().and_then(ComponentRef::id).unwrap_or_default(),
            item: item.try_to_owned()?,
        })
    }

    /// Get the item for the current state of the path.
    pub(crate) fn item(&self) -> &Item {
        &self.item
    }

    /// Push a component and return a guard to it.
    pub(crate) fn push_id(&mut self) -> alloc::Result<Guard> {
        let next_index = self.block_index;
        self.item.push(ComponentRef::Id(next_index))?;
        Ok(Guard(self.item.as_bytes().len()))
    }

    /// Push a component and return a guard to it.
    pub(crate) fn push_name(&mut self, name: &str) -> alloc::Result<Guard> {
        self.block_index = 0;
        self.item.push(name)?;
        Ok(Guard(self.item.as_bytes().len()))
    }

    /// Pop the scope associated with the given guard.
    pub(crate) fn pop(&mut self, Guard(expected): Guard) -> Result<(), ErrorKind> {
        if self.item.as_bytes().len() != expected {
            return Err(ErrorKind::from(GuardMismatch {
                actual: self.item.as_bytes().len(),
                expected,
            }));
        }

        self.block_index = self
            .item
            .pop()?
            .and_then(|c| c.id())
            .and_then(|n| n.checked_add(1))
            .unwrap_or_default();

        Ok(())
    }
}
