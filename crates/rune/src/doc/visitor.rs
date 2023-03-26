use crate::collections::{BTreeMap, HashMap};
use crate::compile::{
    CompileVisitor, IntoComponent, Item, ItemBuf, Location, MetaKind, MetaRef, Names,
};

/// Visitor used to collect documentation from rune sources.
pub struct Visitor {
    pub(crate) base: ItemBuf,
    pub(crate) names: Names,
    pub(crate) meta: BTreeMap<ItemBuf, MetaKind>,
    pub(crate) docs: HashMap<ItemBuf, Vec<String>>,
    pub(crate) field_docs: HashMap<ItemBuf, HashMap<Box<str>, Vec<String>>>,
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
            meta: BTreeMap::default(),
            docs: HashMap::default(),
            field_docs: HashMap::default(),
        }
    }
}

impl CompileVisitor for Visitor {
    fn register_meta(&mut self, meta: MetaRef<'_>) {
        let item = self.base.join(meta.item);
        self.meta.insert(item.to_owned(), meta.kind);
        self.names.insert(item);
    }

    fn visit_doc_comment(&mut self, _location: Location, item: &Item, string: &str) {
        let item = self.base.join(item);
        self.docs.entry(item).or_default().push(string.to_owned());
    }

    fn visit_field_doc_comment(
        &mut self,
        _location: Location,
        item: &Item,
        field: &str,
        string: &str,
    ) {
        let item = self.base.join(item);
        let map = self.field_docs.entry(item).or_default();
        map.entry(field.into()).or_default().push(string.to_owned());
    }
}
