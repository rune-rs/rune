use core::cell::{Ref, RefCell};
use core::fmt;

use crate::no_std::prelude::*;
use crate::no_std::rc::Rc;

use crate::compile::{ComponentRef, Item, ItemBuf};
use crate::parse::NonZeroId;
use crate::shared::Gen;

#[non_exhaustive]
pub(crate) struct MissingLastId;

impl fmt::Display for MissingLastId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "missing last inserted id into the items stack")
    }
}

#[derive(Debug)]
struct Inner<'a> {
    id: usize,
    item: ItemBuf,
    ids: Vec<NonZeroId>,
    gen: &'a Gen,
}

/// Manage item paths.
#[derive(Debug)]
pub(crate) struct Items<'a> {
    inner: Rc<RefCell<Inner<'a>>>,
}

impl<'a> Items<'a> {
    /// Construct a new items manager.
    pub(crate) fn new(item: &Item, gen: &'a Gen) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                id: item.last().and_then(ComponentRef::id).unwrap_or_default(),
                item: item.to_owned(),
                ids: Vec::new(),
                gen,
            })),
        }
    }

    /// Access the last added id.
    pub(crate) fn id(&self) -> Result<NonZeroId, MissingLastId> {
        self.inner.borrow().ids.last().copied().ok_or(MissingLastId)
    }

    /// Get the item for the current state of the path.
    pub(crate) fn item(&self) -> Ref<'_, ItemBuf> {
        Ref::map(self.inner.borrow(), |inner| &inner.item)
    }

    /// Push a component and return a guard to it.
    pub(crate) fn push_id(&self) -> Guard<'a> {
        let mut inner = self.inner.borrow_mut();
        let id = inner.gen.next();

        let next_id = inner.id;
        inner.item.push(ComponentRef::Id(next_id));
        inner.ids.push(id);

        Guard {
            inner: self.inner.clone(),
        }
    }

    /// Push a component and return a guard to it.
    pub(crate) fn push_name(&self, name: &str) -> Guard<'a> {
        let mut inner = self.inner.borrow_mut();
        let id = inner.gen.next();

        inner.id = 0;
        inner.item.push(name);
        inner.ids.push(id);

        Guard {
            inner: self.inner.clone(),
        }
    }
}

pub(crate) struct Guard<'a> {
    inner: Rc<RefCell<Inner<'a>>>,
}

impl<'a> Drop for Guard<'a> {
    fn drop(&mut self) {
        let mut inner = self.inner.borrow_mut();

        let next_id = inner
            .item
            .pop()
            .and_then(|c| c.id())
            .and_then(|n| n.checked_add(1))
            .unwrap_or_default();

        inner.ids.pop();
        inner.id = next_id;
    }
}
