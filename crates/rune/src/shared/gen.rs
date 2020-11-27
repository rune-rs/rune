use runestick::Id;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Default, Debug, Clone)]
pub(crate) struct Gen {
    id: Rc<Cell<Id>>,
}

impl Gen {
    /// Construct a new shared generator.
    pub(crate) fn new() -> Self {
        Self {
            id: Rc::new(Cell::new(Id::initial())),
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
