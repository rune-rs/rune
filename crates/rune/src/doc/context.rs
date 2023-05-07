use crate::no_std::prelude::*;

use crate::compile::context::PrivMeta;
use crate::compile::{meta, ComponentRef, IntoComponent, Item, ItemBuf};
use crate::doc::{Visitor, VisitorData};
use crate::module::{AssociatedKind, ModuleAssociated};
use crate::runtime::ConstValue;
use crate::runtime::Protocol;
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
    Instance(&'a str),
}

/// Information on an associated function.
pub(crate) struct AssocFn<'a> {
    pub(crate) is_async: bool,
    pub(crate) return_type: Option<Hash>,
    pub(crate) argument_types: Box<[Option<Hash>]>,
    pub(crate) docs: &'a [String],
    /// Literal argument replacements.
    /// TODO: replace this with structured information that includes type hash so it can be linked if it's available.
    pub(crate) docs_args: Option<&'a [String]>,
    pub(crate) kind: AssocFnKind<'a>,
    pub(crate) args: Option<usize>,
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
    Unknown,
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
    pub(crate) fn associated(&self, hash: Hash) -> impl Iterator<Item = Assoc<'_>> {
        fn visitor_to_associated(
            hash: Hash,
            visitor: &Visitor,
        ) -> Option<impl Iterator<Item = Assoc<'_>>> {
            let associated = visitor.associated.get(&hash)?;

            Some(associated.iter().flat_map(move |hash| {
                let data = visitor.data.get(hash)?;

                let (is_async, kind, args) = match data.kind {
                    meta::Kind::Function {
                        is_async,
                        instance_function,
                        args,
                        ..
                    } if instance_function => (
                        is_async,
                        AssocFnKind::Instance(data.item.last()?.as_str()?),
                        args,
                    ),
                    _ => return None,
                };

                Some(Assoc::Fn(AssocFn {
                    is_async,
                    return_type: None,
                    argument_types: Box::from([]),
                    docs: &data.docs,
                    docs_args: None,
                    kind,
                    args,
                    parameter_types: &[],
                }))
            }))
        }

        fn context_to_associated(m: &ModuleAssociated) -> Assoc<'_> {
            let kind = match m.name.kind {
                AssociatedKind::Protocol(protocol) => AssocFnKind::Protocol(protocol),
                AssociatedKind::FieldFn(protocol, ref field) => {
                    AssocFnKind::FieldFn(protocol, field)
                }
                AssociatedKind::IndexFn(protocol, index) => {
                    AssocFnKind::IndexFn(protocol, index)
                }
                AssociatedKind::Instance(ref name) => AssocFnKind::Instance(name),
            };

            Assoc::Fn(AssocFn {
                is_async: m.is_async,
                return_type: m.return_type.as_ref().map(|f| f.hash),
                argument_types: m
                    .argument_types
                    .iter()
                    .map(|f| f.as_ref().map(|f| f.hash))
                    .collect(),
                docs: m.docs.lines(),
                docs_args: m.docs.args(),
                kind,
                args: m.args,
                parameter_types: &m.name.parameter_types,
            })
        }

        let visitors = self
            .visitors
            .iter()
            .flat_map(move |v| visitor_to_associated(hash, v).into_iter().flatten());

        let context = self
            .context
            .associated(hash)
            .map(context_to_associated);
        visitors.chain(context)
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
    pub(crate) fn meta_by_hash(&self, hash: Hash) -> Vec<Meta<'_>> {
        let mut out = Vec::new();

        for visitor in self.visitors {
            if let Some(data) = visitor.get_by_hash(hash) {
                out.push(visitor_meta_to_meta(data));
            }
        }

        for meta in self.context.lookup_meta_by_hash(hash) {
            out.extend(self.context_meta_to_meta(meta));
        }

        out
    }

    /// Lookup all meta matching the given item.
    pub(crate) fn meta(&self, item: &Item) -> Vec<Meta<'_>> {
        let mut out = Vec::new();

        for visitor in self.visitors {
            if let Some(data) = visitor.get(item) {
                out.push(visitor_meta_to_meta(data));
            }
        }

        for meta in self.context.lookup_meta(item) {
            out.extend(self.context_meta_to_meta(meta));
        }

        out
    }

    fn context_meta_to_meta(&self, meta: &'a PrivMeta) -> Option<Meta<'a>> {
        let kind = match &meta.kind {
            meta::Kind::Type { .. } => Kind::Unknown,
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
                    argument_types: &f.argument_types,
                })
            }
            meta::Kind::Const { const_value } => Kind::Const(const_value),
            meta::Kind::Macro => Kind::Macro,
            meta::Kind::Module { .. } => Kind::Module,
            _ => Kind::Unsupported,
        };

        Some(Meta {
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

fn visitor_meta_to_meta(data: &VisitorData) -> Meta<'_> {
    let kind = match &data.kind {
        meta::Kind::Type { .. } => Kind::Unknown,
        meta::Kind::Struct { .. } => Kind::Struct,
        meta::Kind::Variant { .. } => Kind::Variant,
        meta::Kind::Enum => Kind::Enum,
        meta::Kind::Function { is_async, args, .. } => Kind::Function(Function {
            is_async: *is_async,
            args: None,
            signature: Signature::Function { args: *args },
            return_type: None,
            argument_types: &[],
        }),
        meta::Kind::Module => Kind::Module,
        _ => Kind::Unsupported,
    };

    Meta {
        item: &data.item,
        hash: data.hash,
        docs: data.docs.as_slice(),
        kind,
    }
}
