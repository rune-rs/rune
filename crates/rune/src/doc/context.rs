use crate::no_std::prelude::*;

use crate::compile::context::PrivMeta;
use crate::compile::meta;
use crate::compile::{AssociatedFunction, ComponentRef, IntoComponent, Item};
use crate::doc::{Visitor, VisitorData};
use crate::runtime::ConstValue;
use crate::Hash;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Meta<'a> {
    /// Item of the meta.
    pub(crate) item: &'a Item,
    /// Type hash for the meta item.
    pub(crate) hash: Hash,
    /// Kind of the meta item.
    pub(crate) kind: Kind<'a>,
    /// Documentation for the meta item.
    pub(crate) docs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Function<'a> {
    pub(crate) is_async: bool,
    pub(crate) args: Option<&'a [String]>,
    pub(crate) signature: Signature,
    pub(crate) return_type: Option<Hash>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Kind<'a> {
    Unsupported,
    Unknown,
    Struct,
    Variant,
    Enum,
    Function(Function<'a>),
    Const(&'a ConstValue),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Signature {
    Function { args: Option<usize> },
    Instance { args: Option<usize> },
}

/// Build context for documentation.
///
/// Provides a unified API for querying information about known types.
pub(crate) struct Context<'a> {
    context: &'a crate::Context,
    visitors: &'a [Visitor],
}

impl<'a> Context<'a> {
    pub(crate) fn new(context: &'a crate::Context, visitors: &'a [Visitor]) -> Self {
        Self { context, visitors }
    }

    /// Iterate over all types associated with the given hash.
    pub(crate) fn associated(&self, hash: Hash) -> impl Iterator<Item = &AssociatedFunction> {
        self.context.associated(hash)
    }

    /// Iterate over known child components of the given name.
    pub(crate) fn iter_components<'iter, I: 'iter>(
        &'iter self,
        iter: I,
    ) -> impl Iterator<Item = ComponentRef<'iter>> + 'iter
    where
        I: Clone + IntoIterator,
        I::Item: IntoComponent,
    {
        let tail = self.context.iter_components(iter.clone());
        self.visitors
            .iter()
            .flat_map(move |v| v.names.iter_components(iter.clone()))
            .chain(tail)
    }

    /// Get a meta item by its hash.
    pub(crate) fn meta_by_hash(&self, hash: Hash) -> Option<Meta<'_>> {
        for visitor in self.visitors {
            if let Some(data) = visitor.get_by_hash(hash) {
                return Some(visitor_meta_to_meta(data));
            }
        }

        let meta = self.context.lookup_meta_by_hash(hash)?;
        self.context_meta_to_meta(meta)
    }

    /// Lookup Meta.
    pub(crate) fn meta(&self, item: &Item) -> Option<Meta<'_>> {
        for visitor in self.visitors {
            if let Some(data) = visitor.get(item) {
                return Some(visitor_meta_to_meta(data));
            }
        }

        let meta = self.context.lookup_meta(item)?;
        self.context_meta_to_meta(meta)
    }

    fn context_meta_to_meta(&self, meta: &'a PrivMeta) -> Option<Meta<'a>> {
        let kind = match &meta.kind {
            meta::Kind::Unknown { .. } => Kind::Unknown,
            meta::Kind::Struct { .. } => Kind::Struct,
            meta::Kind::Variant { .. } => Kind::Variant,
            meta::Kind::Enum { .. } => Kind::Enum,
            meta::Kind::Function {
                args,
                instance_function,
                ..
            } => {
                let f = self.context.lookup_signature(meta.hash)?;

                let instance_function = match f.kind {
                    meta::SignatureKind::Function => *instance_function,
                    meta::SignatureKind::Instance { .. } => true,
                };

                let signature = if instance_function {
                    Signature::Instance { args: *args }
                } else {
                    Signature::Function { args: *args }
                };

                Kind::Function(Function {
                    is_async: f.is_async,
                    signature,
                    args: meta.docs.args(),
                    return_type: f.return_type,
                })
            }
            meta::Kind::Const { const_value } => Kind::Const(const_value),
            _ => Kind::Unsupported,
        };

        let m = Meta {
            item: &meta.item,
            hash: meta.hash,
            docs: meta.docs.lines(),
            kind,
        };

        Some(m)
    }

    /// Iterate over known modules.
    pub(crate) fn iter_modules(&self) -> impl IntoIterator<Item = &Item> {
        self.visitors
            .iter()
            .map(|v| v.base.as_ref())
            .chain(self.context.iter_meta().map(|m| m.module.as_ref()))
    }
}

fn visitor_meta_to_meta(data: &VisitorData) -> Meta<'_> {
    let kind = match &data.kind {
        meta::Kind::Unknown { .. } => Kind::Unknown,
        meta::Kind::Struct { .. } => Kind::Struct,
        meta::Kind::Variant { .. } => Kind::Variant,
        meta::Kind::Enum => Kind::Enum,
        meta::Kind::Function { is_async, args, .. } => Kind::Function(Function {
            is_async: *is_async,
            args: None,
            signature: Signature::Function { args: *args },
            return_type: None,
        }),
        _ => Kind::Unsupported,
    };

    Meta {
        item: &data.item,
        hash: data.hash,
        docs: data.docs.as_slice(),
        kind,
    }
}
