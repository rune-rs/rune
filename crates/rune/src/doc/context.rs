use crate::compile::{
    ComponentRef, ContextMetaKind, ContextSignature, IntoComponent, Item, MetaKind,
};
use crate::doc::Visitor;
use crate::runtime::ConstValue;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Meta<'a> {
    pub(crate) kind: Kind<'a>,
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
    visitor: &'a Visitor,
}

impl<'a> Context<'a> {
    pub(crate) fn new(context: &'a crate::Context, visitor: &'a Visitor) -> Self {
        Self { context, visitor }
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
        self.visitor
            .names
            .iter_components(iter.clone())
            .chain(self.context.iter_components(iter))
    }

    /// Lookup Meta.
    pub(crate) fn meta(&self, item: &Item) -> Option<Meta<'_>> {
        if let Some(m) = self.visitor.meta.get(item) {
            let kind = match m {
                MetaKind::Unknown => Kind::Unknown,
                MetaKind::UnitStruct => Kind::Struct,
                MetaKind::TupleStruct => Kind::Struct,
                MetaKind::Struct => Kind::Struct,
                MetaKind::UnitVariant => Kind::Variant,
                MetaKind::TupleVariant => Kind::Variant,
                MetaKind::StructVariant => Kind::Variant,
                MetaKind::Enum => Kind::Enum,
                MetaKind::Function { args, .. } => Kind::Function {
                    args: None,
                    signature: Signature::Function { args: *args },
                },
                _ => Kind::Unsupported,
            };

            let docs = self
                .visitor
                .docs
                .get(item)
                .map(Vec::as_slice)
                .unwrap_or_default();

            return Some(Meta { docs, kind });
        }

        let meta = self.context.lookup_meta(item)?;

        let kind = match &meta.kind {
            ContextMetaKind::Unknown { .. } => Kind::Unknown,
            ContextMetaKind::Struct { .. } => Kind::Struct,
            ContextMetaKind::Variant { .. } => Kind::Variant,
            ContextMetaKind::Enum { .. } => Kind::Enum,
            ContextMetaKind::Function {
                args,
                type_hash,
                instance_function,
            } => {
                let f = self.context.lookup_signature(*type_hash)?;

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
            ContextMetaKind::Const { const_value } => Kind::Const(const_value),
        };

        Some(Meta {
            docs: meta.docs.lines(),
            kind,
        })
    }

    /// Iterate over known modules.
    pub(crate) fn iter_modules(&self) -> impl IntoIterator<Item = &Item> {
        self.context.iter_meta().map(|(_, m)| m.module.as_ref())
    }
}
