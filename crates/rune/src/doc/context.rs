use crate::alloc::prelude::*;
use crate::alloc::{self, String, Vec};
use crate::compile::context::ContextMeta;
use crate::compile::meta;
use crate::doc::{Visitor, VisitorData};
use crate::item::{ComponentRef, IntoComponent};
use crate::runtime::ConstValue;
use crate::runtime::Protocol;
use crate::{Hash, Item, ItemBuf};

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
    pub(crate) item: &'a Item,
    /// Type hash for the meta item.
    pub(crate) hash: Hash,
    /// The meta source.
    #[allow(unused)]
    pub(crate) source: MetaSource<'a>,
    /// Indicates if the item is deprecated.
    pub(crate) deprecated: Option<&'a str>,
    /// Documentation for the meta item.
    pub(crate) docs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Function<'a> {
    pub(crate) is_async: bool,
    pub(crate) is_test: bool,
    pub(crate) is_bench: bool,
    pub(crate) signature: Signature,
    pub(crate) arguments: Option<&'a [meta::DocArgument]>,
    pub(crate) return_type: &'a meta::DocType,
}

/// The kind of an associated function.
#[derive(Debug)]
pub(crate) enum AssocFnKind<'a> {
    /// A protocol function implemented on the type itself.
    Protocol(&'static Protocol),
    /// A field function with the given protocol.
    FieldFn(&'static Protocol, &'a str),
    /// An index function with the given protocol.
    IndexFn(&'static Protocol, usize),
    /// The instance function refers to the given named instance fn.
    Method(&'a Item, &'a str, Signature),
}

/// Information on an associated function.
#[derive(Debug)]
pub(crate) struct AssocVariant<'a> {
    /// Name of variant.
    pub(crate) name: &'a str,
    /// Documentation for variant.
    pub(crate) docs: &'a [String],
}

/// Information on an associated function.
#[derive(Debug)]
pub(crate) struct AssocFn<'a> {
    pub(crate) kind: AssocFnKind<'a>,
    pub(crate) trait_hash: Option<Hash>,
    pub(crate) is_async: bool,
    pub(crate) arguments: Option<&'a [meta::DocArgument]>,
    pub(crate) return_type: &'a meta::DocType,
    /// Generic instance parameters for function.
    pub(crate) parameter_types: &'a [Hash],
    pub(crate) deprecated: Option<&'a str>,
    pub(crate) docs: &'a [String],
}

/// Information on an associated item.
#[derive(Debug)]
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
    Trait,
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
    pub(crate) fn associated(&self, hash: Hash) -> impl Iterator<Item = Hash> + '_ {
        let visitors = self
            .visitors
            .iter()
            .flat_map(move |v| {
                v.associated
                    .get(&hash)
                    .map(Vec::as_slice)
                    .unwrap_or_default()
            })
            .copied();

        let context = self
            .context
            .into_iter()
            .flat_map(move |c| c.associated(hash));

        visitors.chain(context)
    }

    pub(crate) fn associated_meta(&self, hash: Hash) -> impl Iterator<Item = Assoc<'a>> + '_ {
        let visitors = self
            .visitors
            .iter()
            .flat_map(move |v| visitor_to_associated(v, hash));

        let context = self
            .context
            .into_iter()
            .flat_map(move |c| context_to_associated(c, hash));

        visitors.chain(context)
    }

    /// Iterate over all traits associated with the given hash.
    pub(crate) fn traits(&self, hash: Hash) -> impl Iterator<Item = Hash> + 'a {
        self.context.into_iter().flat_map(move |c| c.traits(hash))
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
    pub(crate) fn meta_by_hash(&self, hash: Hash) -> alloc::Result<Vec<Meta<'a>>> {
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
        let item = meta.item.as_deref()?;

        let kind = match &meta.kind {
            meta::Kind::Type { .. } => Kind::Type,
            meta::Kind::Struct {
                enum_hash: Hash::EMPTY,
                ..
            } => Kind::Struct,
            meta::Kind::Struct { .. } => Kind::Variant,
            meta::Kind::Enum { .. } => Kind::Enum,
            meta::Kind::Function {
                associated: None,
                signature: f,
                is_test,
                is_bench,
                ..
            } => Kind::Function(Function {
                is_async: f.is_async,
                is_test: *is_test,
                is_bench: *is_bench,
                signature: Signature::Function,
                arguments: f.arguments.as_deref(),
                return_type: &f.return_type,
            }),
            meta::Kind::Function {
                associated: Some(..),
                signature: f,
                is_test,
                is_bench,
                ..
            } => Kind::Function(Function {
                is_async: f.is_async,
                is_test: *is_test,
                is_bench: *is_bench,
                signature: Signature::Instance,
                arguments: f.arguments.as_deref(),
                return_type: &f.return_type,
            }),
            meta::Kind::Const => {
                let const_value = self.context?.get_const_value(meta.hash)?;
                Kind::Const(const_value)
            }
            meta::Kind::Macro => Kind::Macro,
            meta::Kind::Module => Kind::Module,
            meta::Kind::Trait => Kind::Trait,
            _ => Kind::Unsupported,
        };

        Some(Meta {
            kind,
            source: MetaSource::Context,
            item,
            hash: meta.hash,
            deprecated: meta.deprecated.as_deref(),
            docs: meta.docs.lines(),
        })
    }

    /// Iterate over known modules.
    pub(crate) fn iter_modules(&self) -> impl IntoIterator<Item = alloc::Result<ItemBuf>> + '_ {
        let visitors = self
            .visitors
            .iter()
            .flat_map(|v| v.base.as_crate().map(ItemBuf::with_crate));

        let contexts = self
            .context
            .into_iter()
            .flat_map(|c| c.iter_crates().map(ItemBuf::with_crate));

        visitors.chain(contexts)
    }
}

fn visitor_to_associated(visitor: &Visitor, hash: Hash) -> impl Iterator<Item = Assoc<'_>> + '_ {
    let associated = visitor.associated.get(&hash).into_iter();

    associated.flat_map(move |a| {
        a.iter().flat_map(move |hash| {
            let data = visitor.data.get(hash)?;

            let (associated, trait_hash, signature) = match data.kind.as_ref()? {
                meta::Kind::Function {
                    associated,
                    trait_hash,
                    signature,
                    ..
                } => (associated, trait_hash, signature),
                meta::Kind::Struct { enum_hash, .. } if *enum_hash != Hash::EMPTY => {
                    return Some(Assoc::Variant(AssocVariant {
                        name: data.item.last()?.as_str()?,
                        docs: &data.docs,
                    }));
                }
                _ => return None,
            };

            let kind = match associated {
                Some(meta::AssociatedKind::Instance(name)) => {
                    AssocFnKind::Method(&data.item, name.as_ref(), Signature::Instance)
                }
                None => AssocFnKind::Method(
                    &data.item,
                    data.item.last()?.as_str()?,
                    Signature::Function,
                ),
                _ => return None,
            };

            Some(Assoc::Fn(AssocFn {
                kind,
                trait_hash: *trait_hash,
                is_async: signature.is_async,
                arguments: signature.arguments.as_deref(),
                return_type: &signature.return_type,
                parameter_types: &[],
                deprecated: data.deprecated.as_deref(),
                docs: &data.docs,
            }))
        })
    })
}

fn context_to_associated(context: &crate::Context, hash: Hash) -> Option<Assoc<'_>> {
    let meta = context.lookup_meta_by_hash(hash).next()?;

    match meta.kind {
        meta::Kind::Struct { enum_hash, .. } if enum_hash != Hash::EMPTY => {
            let name = meta.item.as_deref()?.last()?.as_str()?;

            Some(Assoc::Variant(AssocVariant {
                name,
                docs: meta.docs.lines(),
            }))
        }
        meta::Kind::Function {
            associated: Some(ref associated),
            trait_hash,
            ref parameter_types,
            ref signature,
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
                    AssocFnKind::Method(meta.item.as_ref()?, name, Signature::Instance)
                }
            };

            Some(Assoc::Fn(AssocFn {
                kind,
                trait_hash,
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
            trait_hash,
            ref signature,
            ..
        } => {
            let item = meta.item.as_deref()?;
            let name = item.last()?.as_str()?;
            let kind = AssocFnKind::Method(item, name, Signature::Function);

            Some(Assoc::Fn(AssocFn {
                kind,
                trait_hash,
                is_async: signature.is_async,
                arguments: signature.arguments.as_deref(),
                return_type: &signature.return_type,
                parameter_types: &[],
                deprecated: meta.deprecated.as_deref(),
                docs: meta.docs.lines(),
            }))
        }
        ref _kind => {
            tracing::warn!(kind = ?_kind, "Unsupported associated type");
            None
        }
    }
}

fn visitor_meta_to_meta<'a>(base: &'a Item, data: &'a VisitorData) -> Meta<'a> {
    let kind = match &data.kind {
        Some(meta::Kind::Type { .. }) => Kind::Type,
        Some(meta::Kind::Struct {
            enum_hash: Hash::EMPTY,
            ..
        }) => Kind::Struct,
        Some(meta::Kind::Struct { .. }) => Kind::Variant,
        Some(meta::Kind::Enum { .. }) => Kind::Enum,
        Some(meta::Kind::Function {
            associated,
            signature: f,
            is_test,
            is_bench,
            ..
        }) => Kind::Function(Function {
            is_async: f.is_async,
            is_test: *is_test,
            is_bench: *is_bench,
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
        item: &data.item,
        hash: data.hash,
        deprecated: None,
        docs: data.docs.as_slice(),
        kind,
    }
}
