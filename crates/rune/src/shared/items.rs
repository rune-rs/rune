use runestick::{ComponentRef, Item};
use std::cell::{Ref, RefCell};
use std::rc::Rc;

pub(crate) struct Guard {
    inner: Rc<RefCell<Inner>>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let mut inner = self.inner.borrow_mut();

        let id = inner
            .item
            .pop()
            .and_then(|c| c.id())
            .and_then(|n| n.checked_add(1))
            .unwrap_or_default();

        inner.id = id;
    }
}

#[derive(Debug)]
struct Inner {
    id: usize,
    item: Item,
}

/// Manage item paths.
#[derive(Debug)]
pub(crate) struct Items {
    inner: Rc<RefCell<Inner>>,
}

impl Items {
    /// Construct a new items manager.
    pub(crate) fn new(item: Item) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                id: item.last().and_then(ComponentRef::id).unwrap_or_default(),
                item,
            })),
        }
    }

    /// Check if the current path is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.inner.borrow().item.is_empty()
    }

    /// Push a component and return a guard to it.
    pub(crate) fn push_id(&self) -> Guard {
        let mut inner = self.inner.borrow_mut();

        let id = inner.id;
        inner.item.push(ComponentRef::Id(id));

        Guard {
            inner: self.inner.clone(),
        }
    }

    /// Push a component and return a guard to it.
    pub(crate) fn push_name(&self, name: &str) -> Guard {
        let mut inner = self.inner.borrow_mut();

        inner.id = 0;
        inner.item.push(name);

        Guard {
            inner: self.inner.clone(),
        }
    }

    /// Get the item for the current state of the path.
    pub(crate) fn item(&self) -> Ref<'_, Item> {
        Ref::map(self.inner.borrow(), |inner| &inner.item)
    }
}
