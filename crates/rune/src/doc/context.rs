use crate::no_std::prelude::*;

use crate::compile::context::{ContextMeta, ContextAssociated};
use crate::compile::{meta, ComponentRef, IntoComponent, Item, ItemBuf};
use crate::doc::{Visitor, VisitorData};
use crate::module::AssociatedKind;
use crate::runtime::ConstValue;
use crate::runtime::Protocol;
use crate::Hash;

#[derive(Debug, Clone, Copy)]
pub(crate) enum MetaSource<'a> {
    /// Meta came from context.
    Context,
    /// Meta came from source.
    Source(&'a Item),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Meta<'a> {
    /// Item of the meta.
    pub(crate) item: &'a Item,
    /// The meta source.
    #[allow(unused)]
    pub(crate) source: MetaSource<'a>,
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
    pub(crate) arg_names: Option<&'a [String]>,
    pub(crate) args: Option<usize>,
    pub(crate) signature: Signature,
    pub(crate) return_type: Option<Hash>,
    pub(crate) argument_types: &'a [Option<Hash>],
}

/// The kind of an associated function.
pub(crate) enum AssocFnKind<'a> {
    /// A protocol function implemented on the type itself.
    Protocol(Protocol),
    /// A field function with the given protocol.
    FieldFn(Protocol, &'a str),
    /// An index function with the given protocol.
    IndexFn(Protocol, usize),
    /// The instance function refers to the given named instance fn.
    Method(&'a str, Option<usize>, Signature),
}

/// Information on an associated function.
pub(crate) struct AssocFn<'a> {
    pub(crate) is_async: bool,
    pub(crate) return_type: Option<Hash>,
    pub(crate) argument_types: Box<[Option<Hash>]>,
    pub(crate) docs: &'a [String],
    /// Literal argument replacements.
    /// TODO: replace this with structured information that includes type hash so it can be linked if it's available.
    pub(crate) arg_names: Option<&'a [String]>,
    pub(crate) kind: AssocFnKind<'a>,
    /// Generic instance parameters for function.
    pub(crate) parameter_types: &'a [Hash],
}

/// Information on an associated item.
pub(crate) enum Assoc<'a> {
    /// A variant,
    #[allow(unused)]
    Variant,
    /// A field.
    #[allow(unused)]
    Field,
    /// An associated function.
    Fn(AssocFn<'a>),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Kind<'a> {
    Unsupported,
    Type,
    Struct,
    Variant,
    Enum,
    Macro,
    Function(Function<'a>),
    Const(&'a ConstValue),
    Module,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Signature {
    Function,
    Instance,
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
    pub(crate) fn associated(&self, hash: Hash) -> impl Iterator<Item = Assoc<'a>> {
        fn visitor_to_associated(
            hash: Hash,
            visitor: &Visitor,
        ) -> Option<impl Iterator<Item = Assoc<'_>>> {
            let associated = visitor.associated.get(&hash)?;

            Some(associated.iter().flat_map(move |hash| {
                let data = visitor.data.get(hash)?;

                let (is_async, kind) = match data.kind {
                    meta::Kind::Function {
                        is_async,
                        instance_function,
                        args,
                        ..
                    } => (
                        is_async,
                        AssocFnKind::Method(data.item.last()?.as_str()?, args, if instance_function { Signature::Instance } else { Signature::Function }),
                    ),
                    _ => return None,
                };

                Some(Assoc::Fn(AssocFn {
                    is_async,
                    return_type: None,
                    argument_types: Box::from([]),
                    docs: &data.docs,
                    arg_names: None,
                    kind,
                    parameter_types: &[],
                }))
            }))
        }

        fn context_to_associated<'m>(context: &'m crate::Context, assoc: &'m ContextAssociated) -> Option<Assoc<'m>> {
            match assoc {
                ContextAssociated::Associated(m) => {
                    let kind = match m.name.kind {
                        AssociatedKind::Protocol(protocol) => AssocFnKind::Protocol(protocol),
                        AssociatedKind::FieldFn(protocol, ref field) => {
                            AssocFnKind::FieldFn(protocol, field)
                        }
                        AssociatedKind::IndexFn(protocol, index) => {
                            AssocFnKind::IndexFn(protocol, index)
                        }
                        AssociatedKind::Instance(ref name) => AssocFnKind::Method(name, m.args, Signature::Instance),
                    };

                    Some(Assoc::Fn(AssocFn {
                        is_async: m.is_async,
                        return_type: m.return_type.as_ref().map(|f| f.hash),
                        argument_types: m
                            .argument_types
                            .iter()
                            .map(|f| f.as_ref().map(|f| f.hash))
                            .collect(),
                        docs: m.docs.lines(),
                        arg_names: m.docs.args(),
                        kind,
                        parameter_types: &m.name.parameter_types,
                    }))
                },
                ContextAssociated::Function(hash) => {
                    let meta = context.lookup_meta_by_hash(*hash).iter().next()?;
                    let sig = context.lookup_signature(*hash)?;
                    let name = meta.item.last()?.as_str()?;

                    Some(Assoc::Fn(AssocFn {
                        is_async: sig.is_async,
                        return_type: sig.return_type,
                        argument_types: sig
                            .argument_types.clone(),
                        docs: meta.docs.lines(),
                        arg_names: meta.docs.args(),
                        kind: AssocFnKind::Method(name, sig.args, Signature::Function),
                        parameter_types: &[],
                    }))
                },
            }
        }

        let visitors = self
            .visitors
            .iter()
            .flat_map(move |v| visitor_to_associated(hash, v).into_iter().flatten());

        let context = self
            .context
            .associated(hash)
            .flat_map(|a| context_to_associated(self.context, a));
        visitors.chain(context)
    }

    /// Iterate over known child components of the given name.
    pub(crate) fn iter_components<I>(
        &self,
        iter: I,
    ) -> impl Iterator<Item = (MetaSource<'a>, ComponentRef<'a>)> + 'a
    where
        I: 'a + Clone + IntoIterator,
        I::Item: IntoComponent,
    {
        let tail = self.context.iter_components(iter.clone()).map(|n| (MetaSource::Context, n));
        self.visitors
            .iter()
            .flat_map(move |v| v.names.iter_components(iter.clone()).map(|n| (MetaSource::Source(&v.base), n)))
            .chain(tail)
    }

    /// Get all matching meta items by hash.
    pub(crate) fn meta_by_hash(&self, hash: Hash) -> Vec<Meta<'_>> {
        let mut out = Vec::new();

        for visitor in self.visitors {
            if let Some(data) = visitor.get_by_hash(hash) {
                out.push(visitor_meta_to_meta(&visitor.base, data));
            }
        }

        for meta in self.context.lookup_meta_by_hash(hash) {
            out.extend(self.context_meta_to_meta(meta));
        }

        out
    }

    /// Lookup all meta matching the given item.
    pub(crate) fn meta(&self, item: &Item) -> Vec<Meta<'a>> {
        let mut out = Vec::new();

        for visitor in self.visitors {
            if let Some(data) = visitor.get(item) {
                out.push(visitor_meta_to_meta(&visitor.base, data));
            }
        }

        for meta in self.context.lookup_meta(item) {
            out.extend(self.context_meta_to_meta(meta));
        }

        out
    }

    fn context_meta_to_meta(&self, meta: &'a ContextMeta) -> Option<Meta<'a>> {
        let kind = match &meta.kind {
            meta::Kind::Type { .. } => Kind::Type,
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
                    Signature::Instance
                } else {
                    Signature::Function
                };

                Kind::Function(Function {
                    is_async: f.is_async,
                    signature,
                    arg_names: meta.docs.args(),
                    args: *args,
                    return_type: f.return_type,
                    argument_types: &f.argument_types,
                })
            }
            meta::Kind::Const { const_value } => Kind::Const(const_value),
            meta::Kind::Macro => Kind::Macro,
            meta::Kind::Module { .. } => Kind::Module,
            _ => Kind::Unsupported,
        };

        Some(Meta {
            source: MetaSource::Context,
            item: &meta.item,
            hash: meta.hash,
            docs: meta.docs.lines(),
            kind,
        })
    }

    /// Iterate over known modules.
    pub(crate) fn iter_modules(&self) -> impl IntoIterator<Item = ItemBuf> + '_ {
        self.visitors
            .iter()
            .map(|v| v.base.clone())
            .chain(self.context.iter_crates().map(ItemBuf::with_crate))
    }
}

fn visitor_meta_to_meta<'a>(base: &'a Item, data: &'a VisitorData) -> Meta<'a> {
    let kind = match &data.kind {
        meta::Kind::Type { .. } => Kind::Type,
        meta::Kind::Struct { .. } => Kind::Struct,
        meta::Kind::Variant { .. } => Kind::Variant,
        meta::Kind::Enum => Kind::Enum,
        meta::Kind::Function { is_async, args, .. } => Kind::Function(Function {
            is_async: *is_async,
            arg_names: None,
            args: *args,
            signature: Signature::Function,
            return_type: None,
            argument_types: &[],
        }),
        meta::Kind::Module => Kind::Module,
        _ => Kind::Unsupported,
    };

    Meta {
        source: MetaSource::Source(base),
        item: &data.item,
        hash: data.hash,
        docs: data.docs.as_slice(),
        kind,
    }
}
