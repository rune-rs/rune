use core::fmt;

use crate::no_std::prelude::*;

use crate::compile::{ComponentRef, Item, ItemBuf};
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

impl crate::no_std::error::Error for MissingLastId {}

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

impl crate::no_std::error::Error for GuardMismatch {}

/// Guard returned.
#[must_use]
pub(crate) struct Guard(usize);

/// Manage item paths.
#[derive(Debug)]
pub(crate) struct Items<'a> {
    id: usize,
    item: ItemBuf,
    ids: Vec<NonZeroId>,
    gen: &'a Gen,
}

impl<'a> Items<'a> {
    /// Construct a new items manager.
    pub(crate) fn new(item: &Item, id: NonZeroId, gen: &'a Gen) -> Self {
        Self {
            id: item.last().and_then(ComponentRef::id).unwrap_or_default(),
            item: item.to_owned(),
            ids: vec![id],
            gen,
        }
    }

    /// Access the last added id.
    pub(crate) fn id(&self) -> Result<NonZeroId, MissingLastId> {
        self.ids.last().copied().ok_or(MissingLastId)
    }

    /// Get the item for the current state of the path.
    pub(crate) fn item(&self) -> &ItemBuf {
        &self.item
    }

    /// Push a component and return a guard to it.
    pub(crate) fn push_id(&mut self) -> Guard {
        let id = self.gen.next();
        let next_id = self.id;

        let len = self.ids.len();

        self.item.push(ComponentRef::Id(next_id));
        self.ids.push(id);

        Guard(len)
    }

    /// Push a component and return a guard to it.
    pub(crate) fn push_name(&mut self, name: &str) -> Guard {
        let id = self.gen.next();

        let len = self.ids.len();

        self.id = 0;
        self.item.push(name);
        self.ids.push(id);

        Guard(len)
    }

    /// Pop the scope associated with the given guard.
    pub(crate) fn pop(&mut self, guard: Guard) -> Result<(), GuardMismatch> {
        let next_id = self
            .item
            .pop()
            .and_then(|c| c.id())
            .and_then(|n| n.checked_add(1))
            .unwrap_or_default();

        self.ids.pop();
        self.id = next_id;

        let Guard(len) = guard;

        if self.ids.len() != len {
            return Err(GuardMismatch {
                actual: self.ids.len(),
                expected: len,
            });
        }

        Ok(())
    }
}
