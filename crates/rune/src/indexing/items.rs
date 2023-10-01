use core::fmt;
use core::mem::replace;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::compile::{ComponentRef, ErrorKind, Item, ItemBuf};
use crate::parse::NonZeroId;
use crate::shared::Gen;

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
    actual: NonZeroId,
    expected: NonZeroId,
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
pub(crate) struct Guard(NonZeroId, NonZeroId);

/// Manage item paths.
#[derive(Debug)]
pub(crate) struct Items<'a> {
    block_index: usize,
    item: ItemBuf,
    last_id: NonZeroId,
    gen: &'a Gen,
}

impl<'a> Items<'a> {
    /// Construct a new items manager.
    pub(crate) fn new(item: &Item, id: NonZeroId, gen: &'a Gen) -> alloc::Result<Self> {
        Ok(Self {
            block_index: item.last().and_then(ComponentRef::id).unwrap_or_default(),
            item: item.try_to_owned()?,
            last_id: id,
            gen,
        })
    }

    /// Access the last added id.
    pub(crate) fn id(&self) -> Result<NonZeroId, MissingLastId> {
        Ok(self.last_id)
    }

    /// Get the item for the current state of the path.
    pub(crate) fn item(&self) -> &ItemBuf {
        &self.item
    }

    /// Push a component and return a guard to it.
    pub(crate) fn push_id(&mut self) -> alloc::Result<Guard> {
        let id = self.gen.next();
        let next_index = self.block_index;
        self.item.push(ComponentRef::Id(next_index))?;
        let last_id = replace(&mut self.last_id, id);
        Ok(Guard(id, last_id))
    }

    /// Push a component and return a guard to it.
    pub(crate) fn push_name(&mut self, name: &str) -> alloc::Result<Guard> {
        let id = self.gen.next();
        self.block_index = 0;
        self.item.push(name)?;
        let last_id = replace(&mut self.last_id, id);
        Ok(Guard(id, last_id))
    }

    /// Pop the scope associated with the given guard.
    pub(crate) fn pop(&mut self, guard: Guard) -> Result<(), ErrorKind> {
        let Guard(expected_id, last_id) = guard;

        if self.last_id != expected_id {
            return Err(ErrorKind::from(GuardMismatch {
                actual: self.last_id,
                expected: expected_id,
            }));
        }

        self.block_index = self
            .item
            .pop()?
            .and_then(|c| c.id())
            .and_then(|n| n.checked_add(1))
            .unwrap_or_default();

        self.last_id = last_id;
        Ok(())
    }
}
