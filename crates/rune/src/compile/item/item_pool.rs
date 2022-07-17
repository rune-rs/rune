use core::fmt;

use crate::collections::HashMap;
use crate::compile::item::{Item, ItemBuf};

macro_rules! get {
    ($slf:expr, $id:expr) => {{
        let ItemId(id) = $id;
        let id = usize::try_from(id).expect("id overflow");

        match $slf.items.get(id) {
            Some(item) => item,
            None => panic!("missing item by id {id}"),
        }
    }};
}

macro_rules! alloc {
    ($slf:expr, $item:expr) => {{
        if let Some(id) = $slf.to_id.get($item) {
            *id
        } else {
            let id = ItemId(u32::try_from($slf.items.len()).expect("ran out of item ids"));
            let item = $item.to_owned();
            $slf.items.push(item.clone());
            $slf.to_id.insert(item, id);
            id
        }
    }};
}

/// The identifier of an item.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub(crate) struct ItemId(u32);

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A pool of items.
pub(crate) struct ItemPool {
    items: Vec<ItemBuf>,
    to_id: HashMap<ItemBuf, ItemId>,
}

impl Default for ItemPool {
    fn default() -> Self {
        Self {
            items: vec![ItemBuf::new()],
            to_id: [(ItemBuf::new(), ItemId(0))].into_iter().collect(),
        }
    }
}

impl ItemPool {
    /// Lookup an item by the given identifier.
    pub(crate) fn get(&self, id: ItemId) -> &Item {
        get!(self, id)
    }

    /// Allocate or return an existing item.
    pub(crate) fn alloc<T>(&mut self, item: T) -> ItemId
    where
        T: AsRef<Item>,
    {
        let item = item.as_ref();
        alloc!(self, item)
    }

    /// Get the identifier for the parent of the given id.
    pub(crate) fn parent(&mut self, id: ItemId) -> Option<ItemId> {
        let parent = get!(self, id).parent()?;
        Some(alloc!(self, parent))
    }
}
