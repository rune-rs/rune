use crate::no_std::prelude::*;

use crate::collections::{hash_map, HashMap};
use crate::compile::{
    meta, CompileVisitor, IntoComponent, Item, ItemBuf, Location, MetaRef, Names,
};
use crate::hash::Hash;

pub(crate) struct VisitorData {
    pub(crate) item: ItemBuf,
    pub(crate) hash: Hash,
    pub(crate) kind: meta::Kind,
    pub(crate) docs: Vec<String>,
    pub(crate) field_docs: HashMap<Box<str>, Vec<String>>,
}

impl VisitorData {
    fn new(item: ItemBuf, hash: Hash, kind: meta::Kind) -> Self {
        Self {
            item,
            hash,
            kind,
            docs: Vec::new(),
            field_docs: HashMap::new(),
        }
    }
}

/// Visitor used to collect documentation from rune sources.
pub struct Visitor {
    pub(crate) base: ItemBuf,
    pub(crate) names: Names,
    pub(crate) data: HashMap<Hash, VisitorData>,
    pub(crate) item_to_hash: HashMap<ItemBuf, Hash>,
    /// Associated items.
    pub(crate) associated: HashMap<Hash, Vec<Hash>>,
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
            data: HashMap::default(),
            item_to_hash: HashMap::new(),
            associated: HashMap::new(),
        }
    }

    /// Get meta by item.
    pub(crate) fn get(&self, item: &Item) -> Option<&VisitorData> {
        let hash = self.item_to_hash.get(item)?;
        self.data.get(hash)
    }

    /// Get meta by hash.
    pub(crate) fn get_by_hash(&self, hash: Hash) -> Option<&VisitorData> {
        self.data.get(&hash)
    }
}

impl CompileVisitor for Visitor {
    fn register_meta(&mut self, meta: MetaRef<'_>) {
        let item = self.base.join(meta.item);
        tracing::trace!(?item, "registering meta");

        self.names.insert(&item);
        self.item_to_hash.insert(item.to_owned(), meta.hash);

        match self.data.entry(meta.hash) {
            hash_map::Entry::Occupied(e) => {
                e.into_mut().kind = meta.kind.clone();
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(VisitorData::new(item, meta.hash, meta.kind.clone()));
            }
        }

        if let Some(container) = meta.associated_container {
            self.associated
                .entry(container)
                .or_default()
                .push(meta.hash);
        }
    }

    fn visit_doc_comment(&mut self, _location: Location, item: &Item, hash: Hash, string: &str) {
        // Documentation comments are literal source lines, so they're newline
        // terminated. Since we perform our own internal newlines conversion
        // these need to be trimmed - at least between each doc item.
        fn newlines(c: char) -> bool {
            matches!(c, '\n' | '\r')
        }

        let item = self.base.join(item);
        tracing::trace!(?item, "visiting comment");

        let data = self
            .data
            .entry(hash)
            .or_insert_with(|| VisitorData::new(item.to_owned(), hash, meta::Kind::Unknown));

        data.docs.push(string.trim_end_matches(newlines).to_owned());
    }

    fn visit_field_doc_comment(
        &mut self,
        _location: Location,
        item: &Item,
        hash: Hash,
        field: &str,
        string: &str,
    ) {
        let item = self.base.join(item);
        tracing::trace!(?item, "visiting field comment");

        let data = self
            .data
            .entry(hash)
            .or_insert_with(|| VisitorData::new(item.to_owned(), hash, meta::Kind::Unknown));
        data.field_docs
            .entry(field.into())
            .or_default()
            .push(string.to_owned());
    }
}