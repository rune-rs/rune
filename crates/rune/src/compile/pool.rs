use core::fmt;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::try_vec;
use crate::alloc::{self, HashMap, Vec};
#[cfg(feature = "emit")]
use crate::compile::Location;
use crate::compile::{Item, ItemBuf, Visibility};
use crate::hash::Hash;

/// The identifier of a module.
#[derive(Default, Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[repr(transparent)]
pub(crate) struct ModId(u32);

impl fmt::Display for ModId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The identifier of an item.
#[derive(Default, Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[repr(transparent)]
pub(crate) struct ItemId(u32);

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Module, its item and its visibility.
#[derive(Default, Debug)]
#[non_exhaustive]
pub(crate) struct ModMeta {
    /// The location of the module.
    #[cfg(feature = "emit")]
    pub(crate) location: Location,
    /// The item of the module.
    pub(crate) item: ItemId,
    /// The visibility of the module.
    pub(crate) visibility: Visibility,
    /// The kind of the module.
    pub(crate) parent: Option<ModId>,
}

impl ModMeta {
    /// Test if the module recursively is public.
    pub(crate) fn is_public(&self, pool: &Pool) -> bool {
        let mut current = Some(self);

        while let Some(m) = current.take() {
            if !m.visibility.is_public() {
                return false;
            }

            current = m.parent.map(|id| pool.module(id));
        }

        true
    }
}

macro_rules! alloc_item {
    ($self:expr, $item:expr) => {{
        let item = $item;
        let hash = Hash::type_hash(item);

        match $self.hash_to_item.get(&hash) {
            Some(id) => *id,
            None => {
                let id = ItemId(u32::try_from($self.items.len()).expect("ran out of item ids"));
                let item = $item.try_to_owned()?;
                $self.items.try_push(ItemStorage { hash, item })?;
                $self.hash_to_item.try_insert(hash, id)?;
                id
            }
        }
    }};
}

struct ItemStorage {
    hash: Hash,
    item: ItemBuf,
}

/// A pool of items.
pub(crate) struct Pool {
    modules: Vec<ModMeta>,
    items: Vec<ItemStorage>,
    item_to_mod: HashMap<ItemId, ModId>,
    hash_to_item: HashMap<Hash, ItemId>,
}

impl Pool {
    pub fn new() -> alloc::Result<Self> {
        let root_hash: Hash = Hash::type_hash(Item::new());

        Ok(Self {
            modules: Vec::new(),
            items: try_vec![ItemStorage {
                hash: root_hash,
                item: ItemBuf::new(),
            }],
            item_to_mod: HashMap::new(),
            hash_to_item: HashMap::try_from_iter([(root_hash, ItemId(0))])?,
        })
    }

    /// Lookup an item by the given identifier.
    pub(crate) fn item(&self, id: ItemId) -> &Item {
        &self.item_storage(id).item
    }

    /// Look up the type hash of an item.
    pub(crate) fn item_type_hash(&self, id: ItemId) -> Hash {
        self.item_storage(id).hash
    }

    /// Lookup mod meta by the given identifier.
    pub(crate) fn module(&self, ModId(id): ModId) -> &ModMeta {
        let id = usize::try_from(id).expect("module id overflow");

        match self.modules.get(id) {
            Some(item) => item,
            None => panic!("missing module by id {id}"),
        }
    }

    /// Get the item associated with a module.
    pub(crate) fn module_item(&self, id: ModId) -> &Item {
        let id = self.module(id).item;
        self.item(id)
    }

    /// Get the hash associated with a module item.
    pub(crate) fn module_item_hash(&self, id: ModId) -> Hash {
        let id = self.module(id).item;
        self.item_type_hash(id)
    }

    /// Get by item id.
    pub(crate) fn module_by_item(&self, id: ItemId) -> Option<&ModMeta> {
        Some(self.module(*self.item_to_mod.get(&id)?))
    }

    /// Allocate or return an existing module identifier.
    pub(crate) fn alloc_module(&mut self, item: ModMeta) -> alloc::Result<ModId> {
        if let Some(id) = self.item_to_mod.get(&item.item) {
            return Ok(*id);
        }

        let id = ModId(u32::try_from(self.modules.len()).expect("ran out of item ids"));
        self.item_to_mod.try_insert(item.item, id)?;
        self.modules.try_push(item)?;
        Ok(id)
    }

    /// Allocate or return an existing item.
    pub(crate) fn alloc_item<T>(&mut self, item: T) -> alloc::Result<ItemId>
    where
        T: AsRef<Item>,
    {
        Ok(alloc_item!(self, item.as_ref()))
    }

    /// Map a value into a new item.
    pub(crate) fn try_map_alloc<M>(&mut self, id: ItemId, m: M) -> alloc::Result<Option<ItemId>>
    where
        M: FnOnce(&Item) -> Option<&Item>,
    {
        let Some(item) = m(self.item(id)) else {
            return Ok(None);
        };

        Ok(Some(alloc_item!(self, item)))
    }

    fn item_storage(&self, ItemId(id): ItemId) -> &ItemStorage {
        let id = usize::try_from(id).expect("item id overflow");

        match self.items.get(id) {
            Some(item) => item,
            None => panic!("missing item by id {id}"),
        }
    }
}
