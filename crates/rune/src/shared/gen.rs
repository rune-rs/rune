use crate::parse::Id;
use std::cell::Cell;

#[derive(Default, Debug)]
pub(crate) struct Gen {
    id: Cell<Id>,
}

impl Gen {
    /// Construct a new shared generator.
    pub(crate) fn new() -> Self {
        Self {
            id: Cell::new(Id::initial()),
        }
    }

    /// Get the next identifier.
    pub(crate) fn next(&self) -> Id {
        let id = self.id.get();
        let next = id.next().expect("ran out of ids");
        self.id.set(next);
        id
    }
}
