//! Compiler metadata for Rune.

use core::fmt;

use crate as rune;
use crate::alloc::borrow::Cow;
use crate::alloc::path::Path;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, HashMap, Vec};
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::compile::attrs::Parser;
use crate::compile::{self, Item, ItemId, Location, MetaInfo, ModId, Pool, Visibility};
use crate::hash::Hash;
use crate::parse::{NonZeroId, ResolveContext};
use crate::runtime::{Call, Protocol};

/// A meta reference to an item being compiled.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct MetaRef<'a> {
    /// If the meta comes from the context or not.
    pub context: bool,
    /// The hash of a meta item.
    pub hash: Hash,
    /// The item being described.
    pub item: &'a Item,
    /// The kind of the item.
    pub kind: &'a Kind,
    /// The source of the meta.
    pub source: Option<&'a SourceMeta>,
}

/// Information on a compile sourc.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub struct SourceMeta {
    /// The location of the compile source.
    pub location: Location,
    /// The optional path where the meta is declared.
    pub path: Option<Box<Path>>,
}

/// Doc content for a compiled item.
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
pub(crate) struct Doc {
    #[rune(span)]
    pub(crate) span: Span,
    /// The string content of the doc comment.
    pub(crate) doc_string: ast::LitStr,
}

impl Doc {
    pub(crate) fn collect_from(
        cx: ResolveContext<'_>,
        attrs: &mut Parser,
        attributes: &[ast::Attribute],
    ) -> compile::Result<Vec<Doc>> {
        let docs = attrs
            .parse_all::<crate::compile::attrs::Doc>(cx, attributes)?
            .map(|result| {
                result.map(|(span, doc)| Doc {
                    span: span.span(),
                    doc_string: doc.doc_string,
                })
            })
            .try_collect::<compile::Result<_>>()??;

        Ok(docs)
    }
}

/// Metadata about a compiled unit.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub(crate) struct Meta {
    /// If the meta comes from the context or not.
    pub(crate) context: bool,
    /// Hash of the private metadata.
    pub(crate) hash: Hash,
    /// The item of the returned compile meta.
    pub(crate) item_meta: ItemMeta,
    /// The kind of the compile meta.
    pub(crate) kind: Kind,
    /// The source of the meta.
    pub(crate) source: Option<SourceMeta>,
    /// Hash parameters for meta.
    pub(crate) parameters: Hash,
}

impl Meta {
    /// Get the [Meta] which describes metadata.
    pub(crate) fn info(&self, pool: &Pool) -> alloc::Result<MetaInfo> {
        MetaInfo::new(&self.kind, self.hash, Some(pool.item(self.item_meta.item)))
    }

    /// Get the [MetaRef] which describes this [meta::Meta] object.
    pub(crate) fn as_meta_ref<'a>(&'a self, pool: &'a Pool) -> MetaRef<'a> {
        MetaRef {
            context: self.context,
            hash: self.hash,
            item: pool.item(self.item_meta.item),
            kind: &self.kind,
            source: self.source.as_ref(),
        }
    }

    /// Get the type hash of the base type (the one to type check for) for the
    /// given compile meta.
    ///
    /// Note: Variants cannot be used for type checking, you should instead
    /// compare them against the enum type.
    pub(crate) fn type_hash_of(&self) -> Option<Hash> {
        match &self.kind {
            Kind::Type { .. } => Some(self.hash),
            Kind::Struct { .. } => Some(self.hash),
            Kind::Enum { .. } => Some(self.hash),
            Kind::Function { .. } => Some(self.hash),
            Kind::Closure { .. } => Some(self.hash),
            Kind::AsyncBlock { .. } => Some(self.hash),
            Kind::Variant { .. } => None,
            Kind::Const { .. } => None,
            Kind::ConstFn { .. } => None,
            Kind::Import { .. } => None,
            Kind::Macro => None,
            Kind::AttributeMacro => None,
            Kind::Module => None,
        }
    }
}

/// The kind of a variant.
#[derive(Debug, TryClone)]
pub enum Fields {
    /// Named fields.
    Named(FieldsNamed),
    /// Unnamed fields.
    Unnamed(usize),
    /// Empty.
    Empty,
}

/// Compile-time metadata kind about a unit.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub enum Kind {
    /// The type is completely opaque. We have no idea about what it is with the
    /// exception of it having a type hash.
    Type {
        /// Hash of generic parameters.
        parameters: Hash,
    },
    /// Metadata about a struct.
    Struct {
        /// Fields information.
        fields: Fields,
        /// Native constructor for this struct.
        constructor: Option<Signature>,
        /// Hash of generic parameters.
        parameters: Hash,
    },
    /// Metadata about an empty variant.
    Variant {
        /// Type hash of the enum this unit variant belongs to.
        enum_hash: Hash,
        /// The index of the variant.
        index: usize,
        /// Fields information.
        fields: Fields,
        /// Native constructor for this variant.
        constructor: Option<Signature>,
    },
    /// An enum item.
    Enum {
        /// Hash of generic parameters.
        parameters: Hash,
    },
    /// A macro item.
    Macro,
    /// An attribute macro item.
    AttributeMacro,
    /// A function declaration.
    Function {
        /// The associated kind of the function, if it is an associated
        /// function.
        associated: Option<AssociatedKind>,
        /// Native signature for this function.
        signature: Signature,
        /// Whether this function has a `#[test]` annotation
        is_test: bool,
        /// Whether this function has a `#[bench]` annotation.
        is_bench: bool,
        /// Hash of generic parameters.
        parameters: Hash,
        /// The container of the associated function.
        #[cfg(feature = "doc")]
        container: Option<Hash>,
        /// Parameter types.
        #[cfg(feature = "doc")]
        parameter_types: Vec<Hash>,
    },
    /// A closure.
    Closure {
        /// Runtime calling convention.
        call: Call,
        /// If the closure moves its environment.
        do_move: bool,
    },
    /// An async block.
    AsyncBlock {
        /// Runtime calling convention.
        call: Call,
        /// If the async block moves its environment.
        do_move: bool,
    },
    /// The constant expression.
    Const,
    /// A constant function.
    ConstFn {
        /// Opaque identifier for the constant function.
        id: NonZeroId,
    },
    /// Purely an import.
    Import(Import),
    /// A module.
    Module,
}

impl Kind {
    /// Access the underlying signature of the kind, if available.
    #[cfg(feature = "doc")]
    pub(crate) fn as_signature(&self) -> Option<&Signature> {
        match self {
            Kind::Struct { constructor, .. } => constructor.as_ref(),
            Kind::Variant { constructor, .. } => constructor.as_ref(),
            Kind::Function { signature, .. } => Some(signature),
            _ => None,
        }
    }

    /// Access underlying generic parameters.
    pub(crate) fn as_parameters(&self) -> Hash {
        match self {
            Kind::Function { parameters, .. } => *parameters,
            Kind::Type { parameters, .. } => *parameters,
            Kind::Enum { parameters, .. } => *parameters,
            Kind::Struct { parameters, .. } => *parameters,
            _ => Hash::EMPTY,
        }
    }

    /// Get the associated container of the meta kind.
    #[cfg(feature = "doc")]
    pub(crate) fn associated_container(&self) -> Option<Hash> {
        match self {
            Kind::Variant { enum_hash, .. } => Some(*enum_hash),
            Kind::Function { container, .. } => *container,
            _ => None,
        }
    }
}

/// An imported entry.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Import {
    /// The location of the import.
    pub(crate) location: Location,
    /// The item being imported.
    pub(crate) target: ItemId,
    /// The module in which the imports is located.
    pub(crate) module: ModId,
}

/// Metadata about named fields.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub struct FieldsNamed {
    /// Fields associated with the type.
    pub(crate) fields: HashMap<Box<str>, FieldMeta>,
}

/// Metadata for a single named field.
#[derive(Debug, TryClone)]
pub struct FieldMeta {
    /// Position of the field in its containing type declaration.
    pub(crate) position: usize,
}

/// Item and the module that the item belongs to.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ItemMeta {
    /// The id of the item.
    pub(crate) id: NonZeroId,
    /// The location of the item.
    pub(crate) location: Location,
    /// The name of the item.
    pub(crate) item: ItemId,
    /// The visibility of the item.
    pub(crate) visibility: Visibility,
    /// The module associated with the item.
    pub(crate) module: ModId,
}

impl ItemMeta {
    /// Test if the item is public (and should be exported).
    pub(crate) fn is_public(&self, pool: &Pool) -> bool {
        self.visibility.is_public() && pool.module(self.module).is_public(pool)
    }
}

/// A description of a function signature.
#[derive(Debug, TryClone)]
pub struct Signature {
    /// An asynchronous function.
    #[cfg(feature = "doc")]
    pub(crate) is_async: bool,
    /// Arguments.
    #[cfg(feature = "doc")]
    pub(crate) args: Option<usize>,
    /// Return type of the function.
    #[cfg(feature = "doc")]
    pub(crate) return_type: Option<Hash>,
    /// Argument types to the function.
    #[cfg(feature = "doc")]
    pub(crate) argument_types: Box<[Option<Hash>]>,
}

/// The kind of an associated function.
#[derive(Debug, TryClone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AssociatedKind {
    /// A protocol function implemented on the type itself.
    Protocol(Protocol),
    /// A field function with the given protocol.
    FieldFn(Protocol, Cow<'static, str>),
    /// An index function with the given protocol.
    IndexFn(Protocol, usize),
    /// The instance function refers to the given named instance fn.
    Instance(Cow<'static, str>),
}

impl AssociatedKind {
    /// Convert the kind into a hash function.
    pub(crate) fn hash(&self, instance_type: Hash) -> Hash {
        match self {
            Self::Protocol(protocol) => Hash::associated_function(instance_type, protocol.hash),
            Self::IndexFn(protocol, index) => {
                Hash::index_function(*protocol, instance_type, Hash::index(*index))
            }
            Self::FieldFn(protocol, field) => {
                Hash::field_function(*protocol, instance_type, field.as_ref())
            }
            Self::Instance(name) => Hash::associated_function(instance_type, name.as_ref()),
        }
    }
}

impl fmt::Display for AssociatedKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssociatedKind::Protocol(protocol) => write!(f, "<{}>", protocol.name),
            AssociatedKind::FieldFn(protocol, field) => {
                write!(f, ".{field}<{}>", protocol.name)
            }
            AssociatedKind::IndexFn(protocol, index) => {
                write!(f, ".{index}<{}>", protocol.name)
            }
            AssociatedKind::Instance(name) => write!(f, "{}", name),
        }
    }
}
