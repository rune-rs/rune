use crate::collections::HashMap;
use crate::compile::{
    meta, CompileVisitor, IntoComponent, Item, ItemBuf, Location, MetaRef, Names,
};
use crate::hash::Hash;

/// Visitor used to collect documentation from rune sources.
pub struct Visitor {
    pub(crate) base: ItemBuf,
    pub(crate) names: Names,
    pub(crate) meta: HashMap<Hash, meta::Kind>,
    pub(crate) docs: HashMap<Hash, Vec<String>>,
    pub(crate) field_docs: HashMap<Hash, HashMap<Box<str>, Vec<String>>>,
    pub(crate) item_to_hash: HashMap<ItemBuf, Hash>,
}

impl Visitor {
    /// Construct a visitor with the given base component.
    pub fn new<I>(base: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        Self {
            base: base.into_iter().collect(),
            names: Names::default(),
            meta: HashMap::default(),
            item_to_hash: HashMap::new(),
            docs: HashMap::default(),
            field_docs: HashMap::default(),
        }
    }

    /// Get meta by item.
    pub(crate) fn get(&self, item: &Item) -> Option<(Hash, &meta::Kind)> {
        let hash = self.item_to_hash.get(item)?;
        Some((*hash, self.meta.get(hash)?))
    }

    /// Get meta by hash.
    pub(crate) fn get_by_hash(&self, hash: Hash) -> Option<&meta::Kind> {
        self.meta.get(&hash)
    }
}

impl CompileVisitor for Visitor {
    fn register_meta(&mut self, meta: MetaRef<'_>) {
        let item = self.base.join(meta.item);
        self.item_to_hash.insert(item.to_owned(), meta.hash);
        self.meta.insert(meta.hash, meta.kind.clone());
        self.names.insert(item);
    }

    fn visit_doc_comment(&mut self, _location: Location, item: &Item, string: &str) {
        let item = self.base.join(item);

        if let Some(hash) = self.item_to_hash.get(&item) {
            self.docs.entry(*hash).or_default().push(string.to_owned());
        }
    }

    fn visit_field_doc_comment(
        &mut self,
        _location: Location,
        item: &Item,
        field: &str,
        string: &str,
    ) {
        let item = self.base.join(item);

        if let Some(hash) = self.item_to_hash.get(&item) {
            let map = self.field_docs.entry(*hash).or_default();
            map.entry(field.into()).or_default().push(string.to_owned());
        }
    }
}
