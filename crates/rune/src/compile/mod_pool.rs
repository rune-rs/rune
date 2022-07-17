use core::fmt;

use crate::collections::HashMap;
use crate::compile::{ItemId, Location, Visibility};

/// The identifier of a module.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub(crate) struct ModId(u32);

impl fmt::Display for ModId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Module, its item and its visibility.
#[derive(Default, Debug)]
#[non_exhaustive]
pub(crate) struct ModMeta {
    /// The location of the module.
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
    pub(crate) fn is_public(&self, mod_pool: &ModPool) -> bool {
        let mut current = Some(self);

        while let Some(m) = current.take() {
            if !m.visibility.is_public() {
                return false;
            }

            current = m.parent.map(|id| mod_pool.get(id));
        }

        true
    }
}

/// A pool of items.
#[derive(Default)]
pub(crate) struct ModPool {
    modules: Vec<ModMeta>,
    to_id: HashMap<ItemId, ModId>,
}

impl ModPool {
    /// Lookup mod meta by the given identifier.
    pub(crate) fn get(&self, ModId(id): ModId) -> &ModMeta {
        let id = usize::try_from(id).expect("id overflow");

        match self.modules.get(id) {
            Some(item) => item,
            None => panic!("missing item by id {id}"),
        }
    }

    /// Get by item id.
    pub(crate) fn by_item(&self, id: ItemId) -> Option<&ModMeta> {
        Some(self.get(*self.to_id.get(&id)?))
    }

    /// Allocate or return an existing module identifier.
    pub(crate) fn alloc(&mut self, item: ModMeta) -> ModId {
        if let Some(id) = self.to_id.get(&item.item) {
            return *id;
        }

        let id = ModId(u32::try_from(self.modules.len()).expect("ran out of item ids"));
        self.to_id.insert(item.item, id);
        self.modules.push(item);
        id
    }
}
