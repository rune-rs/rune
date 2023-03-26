use crate::collections::{BTreeMap, HashMap};
use crate::compile::{CompileVisitor, Item, ItemBuf, Location, MetaKind, MetaRef, Names};

/// Visitor used to collect documentation from rune sources.
#[derive(Default)]
pub struct Visitor {
    pub(crate) names: Names,
    pub(crate) meta: BTreeMap<ItemBuf, MetaKind>,
    pub(crate) docs: HashMap<ItemBuf, Vec<String>>,
    pub(crate) field_docs: HashMap<ItemBuf, HashMap<Box<str>, Vec<String>>>,
}

impl CompileVisitor for Visitor {
    fn register_meta(&mut self, meta: MetaRef<'_>) {
        self.meta.insert(meta.item.to_owned(), meta.kind);
        self.names.insert(meta.item);
    }

    fn visit_doc_comment(&mut self, _location: Location, item: &Item, string: &str) {
        self.docs
            .entry(item.to_owned())
            .or_default()
            .push(string.to_owned());
    }

    fn visit_field_doc_comment(
        &mut self,
        _location: Location,
        item: &Item,
        field: &str,
        string: &str,
    ) {
        let map = self.field_docs.entry(item.to_owned()).or_default();
        map.entry(field.into()).or_default().push(string.to_owned());
    }
}
