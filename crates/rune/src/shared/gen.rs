use core::cell::Cell;
use core::num::NonZeroU32;

use crate::parse::NonZeroId;

#[derive(Debug)]
pub(crate) struct Gen {
    id: Cell<u32>,
}

impl Gen {
    /// Construct a new shared generator.
    pub(crate) fn new() -> Self {
        Self { id: Cell::new(0) }
    }

    /// Get the next id.
    pub(crate) fn next(&self) -> NonZeroId {
        let cur = self.id.get();
        let id = cur
            .checked_add(1)
            .and_then(NonZeroU32::new)
            .expect("ran out of ids");
        self.id.set(id.get());
        NonZeroId::from(id)
    }
}

impl Default for Gen {
    fn default() -> Self {
        Self::new()
    }
}
