use crate::compile::meta;
use crate::compile::{CompileVisitor, ItemBuf, MetaRef};
use crate::Hash;

/// Attribute to collect.
#[derive(Debug, Clone, Copy)]
pub(super) enum Attribute {
    /// Do not collect any functions.
    None,
    /// Collect `#[test]` functions.
    Test,
    /// Collect `#[bench]` functions.
    Bench,
}

/// A compile visitor that collects functions with a specific attribute.
pub(super) struct FunctionVisitor {
    attribute: Attribute,
    functions: Vec<(Hash, ItemBuf)>,
}

impl FunctionVisitor {
    pub(super) fn new(kind: Attribute) -> Self {
        Self {
            attribute: kind,
            functions: Default::default(),
        }
    }

    /// Convert visitor into test functions.
    pub(super) fn into_functions(self) -> Vec<(Hash, ItemBuf)> {
        self.functions
    }
}

impl CompileVisitor for FunctionVisitor {
    fn register_meta(&mut self, meta: MetaRef<'_>) {
        let type_hash = match (self.attribute, &meta.kind) {
            (Attribute::Test, meta::Kind::Function { is_test, .. }) if *is_test => meta.hash,
            (Attribute::Bench, meta::Kind::Function { is_bench, .. }) if *is_bench => meta.hash,
            _ => return,
        };

        self.functions.push((type_hash, meta.item.to_owned()));
    }
}
