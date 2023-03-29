use crate::compile::{meta, ComponentRef, ContextSignature, IntoComponent, Item};
use crate::doc::Visitor;
use crate::runtime::ConstValue;
use crate::Hash;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Meta<'a> {
    /// Type hash for the meta item.
    pub(crate) hash: Hash,
    /// Kind of the meta item.
    pub(crate) kind: Kind<'a>,
    /// Documentation for the meta item.
    pub(crate) docs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Kind<'a> {
    Unsupported,
    Unknown,
    Struct,
    Variant,
    Enum,
    Function {
        args: Option<&'a [String]>,
        signature: Signature,
    },
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
    pub(crate) fn associated(&self, hash: Hash) -> impl Iterator<Item = &Item> {
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

    /// Lookup Meta.
    pub(crate) fn meta(&self, item: &Item) -> Option<Meta<'_>> {
        for visitor in self.visitors {
            if let Some(m) = visitor.meta.get(item) {
                let kind = match m {
                    meta::Kind::Unknown { .. } => Kind::Unknown,
                    meta::Kind::Struct { .. } => Kind::Struct,
                    meta::Kind::Variant { .. } => Kind::Variant,
                    meta::Kind::Enum => Kind::Enum,
                    meta::Kind::Function { args, .. } => Kind::Function {
                        args: None,
                        signature: Signature::Function { args: *args },
                    },
                    _ => Kind::Unsupported,
                };

                let docs = visitor
                    .docs
                    .get(item)
                    .map(Vec::as_slice)
                    .unwrap_or_default();

                return Some(Meta {
                    hash: Hash::type_hash(item),
                    docs,
                    kind,
                });
            }
        }

        let meta = self.context.lookup_meta(item)?;

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

                let instance_function = match *f {
                    ContextSignature::Function { .. } => *instance_function,
                    ContextSignature::Instance { .. } => true,
                };

                let signature = if instance_function {
                    Signature::Instance { args: *args }
                } else {
                    Signature::Function { args: *args }
                };

                Kind::Function {
                    signature,
                    args: meta.docs.args(),
                }
            }
            meta::Kind::Const { const_value } => Kind::Const(const_value),
            _ => Kind::Unsupported,
        };

        Some(Meta {
            hash: meta.hash,
            docs: meta.docs.lines(),
            kind,
        })
    }

    /// Iterate over known modules.
    pub(crate) fn iter_modules(&self) -> impl IntoIterator<Item = &Item> {
        self.visitors
            .iter()
            .map(|v| v.base.as_ref())
            .chain(self.context.iter_meta().map(|(_, m)| m.module.as_ref()))
    }
}
