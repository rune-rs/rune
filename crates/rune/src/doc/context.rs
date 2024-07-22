use crate::alloc::prelude::*;
use crate::alloc::{self, String, Vec};
use crate::compile::context::ContextMeta;
use crate::compile::{meta, ComponentRef, IntoComponent, Item, ItemBuf};
use crate::doc::{Visitor, VisitorData};
use crate::runtime::ConstValue;
use crate::runtime::Protocol;
use crate::Hash;

#[derive(Debug, Clone, Copy)]
pub(crate) enum MetaSource<'a> {
    /// Meta came from context.
    Context,
    /// Meta came from source.
    Source(#[allow(unused)] &'a Item),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Meta<'a> {
    /// Kind of the meta item.
    pub(crate) kind: Kind<'a>,
    /// Item of the meta.
    pub(crate) item: Option<&'a Item>,
    /// The meta source.
    #[allow(unused)]
    pub(crate) source: MetaSource<'a>,
    /// Type hash for the meta item.
    pub(crate) hash: Hash,
    /// Indicates if the item is deprecated.
    pub(crate) deprecated: Option<&'a str>,
    /// Documentation for the meta item.
    pub(crate) docs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Function<'a> {
    pub(crate) is_async: bool,
    pub(crate) signature: Signature,
    pub(crate) arguments: Option<&'a [meta::DocArgument]>,
    pub(crate) return_type: &'a meta::DocType,
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
    Method(&'a str, Signature),
}

/// Information on an associated function.
pub(crate) struct AssocVariant<'a> {
    /// Name of variant.
    pub(crate) name: &'a str,
    /// Documentation for variant.
    pub(crate) docs: &'a [String],
}

/// Information on an associated function.
pub(crate) struct AssocFn<'a> {
    pub(crate) kind: AssocFnKind<'a>,
    pub(crate) is_async: bool,
    pub(crate) arguments: Option<&'a [meta::DocArgument]>,
    pub(crate) return_type: &'a meta::DocType,
    /// Generic instance parameters for function.
    pub(crate) parameter_types: &'a [Hash],
    pub(crate) deprecated: Option<&'a str>,
    pub(crate) docs: &'a [String],
}

/// Information on an associated item.
pub(crate) enum Assoc<'a> {
    /// A variant,
    Variant(AssocVariant<'a>),
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
    Const(#[allow(unused)] &'a ConstValue),
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
    context: Option<&'a crate::Context>,
    visitors: &'a [Visitor],
}

impl<'a> Context<'a> {
    pub(crate) fn new(context: Option<&'a crate::Context>, visitors: &'a [Visitor]) -> Self {
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

                let (is_async, kind, arguments, return_type) = match &data.kind {
                    Some(meta::Kind::Function {
                        associated: None,
                        signature: f,
                        ..
                    }) => (
                        f.is_async,
                        AssocFnKind::Method(data.item.last()?.as_str()?, Signature::Function),
                        f.arguments.as_deref(),
                        &f.return_type,
                    ),
                    Some(meta::Kind::Function {
                        associated: Some(meta::AssociatedKind::Instance(name)),
                        signature: f,
                        ..
                    }) => (
                        f.is_async,
                        AssocFnKind::Method(name.as_ref(), Signature::Instance),
                        f.arguments.as_deref(),
                        &f.return_type,
                    ),
                    Some(meta::Kind::Variant { .. }) => {
                        return Some(Assoc::Variant(AssocVariant {
                            name: data.item.last()?.as_str()?,
                            docs: &data.docs,
                        }));
                    }
                    _ => return None,
                };

                Some(Assoc::Fn(AssocFn {
                    kind,
                    is_async,
                    arguments,
                    return_type,
                    parameter_types: &[],
                    deprecated: data.deprecated.as_deref(),
                    docs: &data.docs,
                }))
            }))
        }

        fn context_to_associated(context: &crate::Context, hash: Hash) -> Option<Assoc<'_>> {
            let meta = context.lookup_meta_by_hash(hash).next()?;

            match &meta.kind {
                meta::Kind::Variant { .. } => {
                    let name = meta.item.as_deref()?.last()?.as_str()?;
                    Some(Assoc::Variant(AssocVariant {
                        name,
                        docs: meta.docs.lines(),
                    }))
                }
                meta::Kind::Function {
                    associated: Some(associated),
                    parameter_types,
                    signature,
                    ..
                } => {
                    let kind = match *associated {
                        meta::AssociatedKind::Protocol(protocol) => AssocFnKind::Protocol(protocol),
                        meta::AssociatedKind::FieldFn(protocol, ref field) => {
                            AssocFnKind::FieldFn(protocol, field)
                        }
                        meta::AssociatedKind::IndexFn(protocol, index) => {
                            AssocFnKind::IndexFn(protocol, index)
                        }
                        meta::AssociatedKind::Instance(ref name) => {
                            AssocFnKind::Method(name, Signature::Instance)
                        }
                    };

                    Some(Assoc::Fn(AssocFn {
                        kind,
                        is_async: signature.is_async,
                        arguments: signature.arguments.as_deref(),
                        return_type: &signature.return_type,
                        parameter_types: &parameter_types[..],
                        deprecated: meta.deprecated.as_deref(),
                        docs: meta.docs.lines(),
                    }))
                }
                meta::Kind::Function {
                    associated: None,
                    signature,
                    ..
                } => {
                    let name = meta.item.as_deref()?.last()?.as_str()?;
                    let kind = AssocFnKind::Method(name, Signature::Function);

                    Some(Assoc::Fn(AssocFn {
                        kind,
                        is_async: signature.is_async,
                        arguments: signature.arguments.as_deref(),
                        return_type: &signature.return_type,
                        parameter_types: &[],
                        deprecated: meta.deprecated.as_deref(),
                        docs: meta.docs.lines(),
                    }))
                }
                kind => {
                    tracing::warn!(?kind, "Unsupported associated type");
                    None
                }
            }
        }

        let visitors = self
            .visitors
            .iter()
            .flat_map(move |v| visitor_to_associated(hash, v).into_iter().flatten());

        let context = self
            .context
            .into_iter()
            .flat_map(move |c| c.associated(hash).flat_map(|a| context_to_associated(c, a)));

        visitors.chain(context)
    }

    /// Iterate over known child components of the given name.
    pub(crate) fn iter_components<I>(
        &self,
        iter: I,
    ) -> alloc::Result<impl Iterator<Item = (MetaSource<'a>, ComponentRef<'a>)> + 'a>
    where
        I: 'a + Clone + IntoIterator,
        I::Item: IntoComponent,
    {
        let mut out = Vec::new();

        if let Some(context) = self.context {
            for c in context.iter_components(iter.clone())? {
                out.try_push((MetaSource::Context, c))?;
            }
        }

        for v in self.visitors {
            for c in v.names.iter_components(iter.clone())? {
                out.try_push((MetaSource::Source(&v.base), c))?;
            }
        }

        Ok(out.into_iter())
    }

    /// Get all matching meta items by hash.
    pub(crate) fn meta_by_hash(&self, hash: Hash) -> alloc::Result<Vec<Meta<'_>>> {
        let mut out = Vec::new();

        for visitor in self.visitors {
            if let Some(data) = visitor.get_by_hash(hash) {
                out.try_push(visitor_meta_to_meta(&visitor.base, data))?;
            }
        }

        if let Some(context) = self.context {
            for meta in context.lookup_meta_by_hash(hash) {
                out.try_extend(self.context_meta_to_meta(meta))?;
            }
        }

        Ok(out)
    }

    /// Lookup all meta matching the given item.
    pub(crate) fn meta(&self, item: &Item) -> alloc::Result<Vec<Meta<'a>>> {
        let mut out = Vec::new();

        for visitor in self.visitors {
            if let Some(data) = visitor.get(item) {
                out.try_push(visitor_meta_to_meta(&visitor.base, data))?;
            }
        }

        if let Some(context) = self.context {
            for meta in context.lookup_meta(item).into_iter().flatten() {
                out.try_extend(self.context_meta_to_meta(meta))?;
            }
        }

        Ok(out)
    }

    fn context_meta_to_meta(&self, meta: &'a ContextMeta) -> Option<Meta<'a>> {
        let kind = match &meta.kind {
            meta::Kind::Type { .. } => Kind::Type,
            meta::Kind::Struct { .. } => Kind::Struct,
            meta::Kind::Variant { .. } => Kind::Variant,
            meta::Kind::Enum { .. } => Kind::Enum,
            meta::Kind::Function {
                associated: None,
                signature: f,
                ..
            } => Kind::Function(Function {
                is_async: f.is_async,
                signature: Signature::Function,
                arguments: f.arguments.as_deref(),
                return_type: &f.return_type,
            }),
            meta::Kind::Function {
                associated: Some(..),
                signature: f,
                ..
            } => Kind::Function(Function {
                is_async: f.is_async,
                signature: Signature::Instance,
                arguments: f.arguments.as_deref(),
                return_type: &f.return_type,
            }),
            meta::Kind::Const { .. } => {
                let const_value = self.context?.get_const_value(meta.hash)?;
                Kind::Const(const_value)
            }
            meta::Kind::Macro => Kind::Macro,
            meta::Kind::Module { .. } => Kind::Module,
            _ => Kind::Unsupported,
        };

        Some(Meta {
            kind,
            source: MetaSource::Context,
            item: meta.item.as_deref(),
            hash: meta.hash,
            deprecated: meta.deprecated.as_deref(),
            docs: meta.docs.lines(),
        })
    }

    /// Iterate over known modules.
    pub(crate) fn iter_modules(&self) -> impl IntoIterator<Item = alloc::Result<ItemBuf>> + '_ {
        self.visitors.iter().map(|v| v.base.try_clone()).chain(
            self.context
                .into_iter()
                .flat_map(|c| c.iter_crates().map(ItemBuf::with_crate)),
        )
    }
}

fn visitor_meta_to_meta<'a>(base: &'a Item, data: &'a VisitorData) -> Meta<'a> {
    let kind = match &data.kind {
        Some(meta::Kind::Type { .. }) => Kind::Type,
        Some(meta::Kind::Struct { .. }) => Kind::Struct,
        Some(meta::Kind::Variant { .. }) => Kind::Variant,
        Some(meta::Kind::Enum { .. }) => Kind::Enum,
        Some(meta::Kind::Function {
            associated,
            signature: f,
            ..
        }) => Kind::Function(Function {
            is_async: f.is_async,
            signature: match associated {
                Some(..) => Signature::Instance,
                None => Signature::Function,
            },
            arguments: f.arguments.as_deref(),
            return_type: &f.return_type,
        }),
        Some(meta::Kind::Module) => Kind::Module,
        _ => Kind::Unsupported,
    };

    Meta {
        source: MetaSource::Source(base),
        item: Some(&data.item),
        hash: data.hash,
        deprecated: None,
        docs: data.docs.as_slice(),
        kind,
    }
}
