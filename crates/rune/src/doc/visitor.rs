use crate::alloc;
use crate::alloc::hash_map::{self, HashMap};
use crate::alloc::prelude::*;
use crate::alloc::{Box, String, Vec};
use crate::compile::meta;
use crate::compile::{CompileVisitor, Located, MetaError, MetaRef, Names};
use crate::item::IntoComponent;
use crate::{Hash, Item, ItemBuf};

pub(crate) struct VisitorData {
    pub(crate) item: ItemBuf,
    pub(crate) hash: Hash,
    pub(crate) kind: Option<meta::Kind>,
    pub(crate) deprecated: Option<String>,
    pub(crate) docs: Vec<String>,
    pub(crate) field_docs: HashMap<Box<str>, Vec<String>>,
}

impl VisitorData {
    fn new(item: ItemBuf, hash: Hash, kind: Option<meta::Kind>) -> Self {
        Self {
            item,
            hash,
            kind,
            deprecated: None,
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
    pub fn new(base: impl IntoIterator<Item: IntoComponent>) -> alloc::Result<Self> {
        let mut this = Self {
            base: base.into_iter().try_collect::<ItemBuf>()?,
            names: Names::default(),
            data: HashMap::default(),
            item_to_hash: HashMap::new(),
            associated: HashMap::new(),
        };

        this.names.insert(&this.base)?;

        let mut it = this.base.iter();

        while !it.as_item().is_empty() {
            let hash = Hash::type_hash(it.as_item());

            this.data.try_insert(
                hash,
                VisitorData::new(it.as_item().try_to_owned()?, hash, Some(meta::Kind::Module)),
            )?;

            this.item_to_hash
                .try_insert(it.as_item().try_to_owned()?, hash)?;
            it.next_back();
        }

        Ok(this)
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

    fn get_or_insert(&mut self, item: &Item) -> Result<&mut VisitorData, MetaError> {
        let item = self.base.join(item)?;
        let hash = Hash::type_hash(&item);

        tracing::trace!(?item, ?hash, "getting");

        let data = match self.data.entry(hash) {
            hash_map::Entry::Occupied(e) => e.into_mut(),
            hash_map::Entry::Vacant(e) => {
                e.try_insert(VisitorData::new(item.try_to_owned()?, hash, None))?
            }
        };

        Ok(data)
    }
}

impl CompileVisitor for Visitor {
    fn register_meta(&mut self, meta: MetaRef<'_>) -> Result<(), MetaError> {
        // Skip over context meta, since we pick that up separately.
        if meta.context {
            return Ok(());
        }

        let item = self.base.join(meta.item)?;
        let hash = Hash::type_hash(&item);

        tracing::trace!(base = ?self.base, meta = ?meta.item, ?item, ?hash, "register meta");

        self.names.insert(&item)?;
        self.item_to_hash.try_insert(item.try_to_owned()?, hash)?;

        match self.data.entry(hash) {
            hash_map::Entry::Occupied(e) => {
                e.into_mut().kind = Some(meta.kind.try_clone()?);
            }
            hash_map::Entry::Vacant(e) => {
                e.try_insert(VisitorData::new(item, hash, Some(meta.kind.try_clone()?)))?;
            }
        }

        if let Some(container) = meta.kind.associated_container() {
            self.associated
                .entry(container)
                .or_try_default()?
                .try_push(hash)?;
        }

        Ok(())
    }

    fn visit_doc_comment(
        &mut self,
        _location: &dyn Located,
        item: &Item,
        _: Hash,
        string: &str,
    ) -> Result<(), MetaError> {
        let data = self.get_or_insert(item)?;

        data.docs
            .try_push(string.trim_end_matches(newlines).try_to_owned()?)?;

        Ok(())
    }

    fn visit_field_doc_comment(
        &mut self,
        _location: &dyn Located,
        item: &Item,
        _: Hash,
        field: &str,
        string: &str,
    ) -> Result<(), MetaError> {
        let data = self.get_or_insert(item)?;

        data.field_docs
            .entry(field.try_into()?)
            .or_try_default()?
            .try_push(string.trim_end_matches(newlines).try_to_owned()?)?;

        Ok(())
    }
}

// Documentation comments are literal source lines, so they're newline
// terminated. Since we perform our own internal newlines conversion
// these need to be trimmed - at least between each doc item.
fn newlines(c: char) -> bool {
    matches!(c, '\n' | '\r')
}
