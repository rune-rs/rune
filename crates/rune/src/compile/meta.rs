//! Compiler metadata for Rune.

use core::fmt;

use crate as rune;
use crate::alloc::borrow::Cow;
use crate::alloc::path::Path;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, Vec};
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::compile::attrs::Parser;
#[cfg(feature = "doc")]
use crate::compile::meta;
use crate::compile::{self, ItemId, Location, MetaInfo, ModId, Pool, Visibility};
use crate::module::{DocFunction, ModuleItemCommon};
use crate::parse::ResolveContext;
use crate::runtime::{Call, FieldMap, Protocol};
use crate::{Hash, Item, ItemBuf};

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
            Kind::Struct {
                enum_hash: Hash::EMPTY,
                ..
            } => Some(self.hash),
            Kind::Struct { .. } => None,
            Kind::Enum { .. } => Some(self.hash),
            Kind::Function { .. } => Some(self.hash),
            Kind::Closure { .. } => Some(self.hash),
            Kind::AsyncBlock { .. } => Some(self.hash),
            Kind::Const { .. } => None,
            Kind::ConstFn { .. } => None,
            Kind::Macro => None,
            Kind::AttributeMacro => None,
            Kind::Import { .. } => None,
            Kind::Alias { .. } => None,
            Kind::Module => None,
            Kind::Trait => None,
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

impl Fields {
    /// Coerce into a tuple field count.
    pub(crate) fn as_tuple(&self) -> Option<usize> {
        match *self {
            Fields::Unnamed(count) => Some(count),
            Fields::Empty => Some(0),
            _ => None,
        }
    }
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
        /// If this is a variant, this is the type hash of the enum.
        ///
        /// If this is not a variant, this is [Hash::EMPTY].
        enum_hash: Hash,
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
        /// The hash of the trait this function is associated with.
        trait_hash: Option<Hash>,
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
    ConstFn,
    /// Purely an import.
    Import(Import),
    /// A re-export.
    Alias(Alias),
    /// A module.
    Module,
    /// A trait.
    Trait,
}

impl Kind {
    /// Access the underlying signature of the kind, if available.
    #[cfg(all(feature = "doc", any(feature = "languageserver", feature = "cli")))]
    pub(crate) fn as_signature(&self) -> Option<&Signature> {
        match self {
            Kind::Struct { constructor, .. } => constructor.as_ref(),
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
        match *self {
            Kind::Struct { enum_hash, .. } if enum_hash != Hash::EMPTY => Some(enum_hash),
            Kind::Function { container, .. } => container,
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
    /// The module in which the imports are located.
    pub(crate) module: ModId,
}

/// A context alias.
#[derive(Debug, TryClone)]
pub struct Alias {
    /// The item being aliased.
    pub(crate) to: ItemBuf,
}

/// Metadata about named fields.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub struct FieldsNamed {
    /// Fields associated with the type.
    pub(crate) fields: Box<[FieldMeta]>,
}

impl FieldsNamed {
    /// Coerce into a hashmap of fields.
    pub(crate) fn to_fields(&self) -> alloc::Result<FieldMap<Box<str>, usize>> {
        let mut fields = crate::runtime::new_field_hash_map_with_capacity(self.fields.len())?;

        for f in self.fields.iter() {
            fields.try_insert(f.name.try_clone()?, f.position)?;
        }

        Ok(fields)
    }
}

/// Metadata for a single named field.
#[derive(Debug, TryClone)]
pub struct FieldMeta {
    /// Position of the field in its containing type declaration.
    pub(crate) name: Box<str>,
    /// The position of the field.
    pub(crate) position: usize,
}

/// Item and the module that the item belongs to.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ItemMeta {
    /// The location of the item.
    pub(crate) location: Location,
    /// The name of the item.
    pub(crate) item: ItemId,
    /// The visibility of the item.
    pub(crate) visibility: Visibility,
    /// The module associated with the item.
    pub(crate) module: ModId,
    /// The impl item associated with the item.
    pub(crate) impl_item: Option<ItemId>,
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
    /// Arguments to the function.
    #[cfg(feature = "doc")]
    pub(crate) arguments: Option<Box<[DocArgument]>>,
    /// Return type of the function.
    #[cfg(feature = "doc")]
    pub(crate) return_type: DocType,
}

impl Signature {
    /// Construct a signature from context metadata.
    #[cfg_attr(not(feature = "doc"), allow(unused_variables))]
    pub(crate) fn from_context(
        doc: &DocFunction,
        common: &ModuleItemCommon,
    ) -> alloc::Result<Self> {
        Ok(Self {
            #[cfg(feature = "doc")]
            is_async: doc.is_async,
            #[cfg(feature = "doc")]
            arguments: context_to_arguments(
                doc.args,
                doc.argument_types.as_ref(),
                common.docs.args(),
            )?,
            #[cfg(feature = "doc")]
            return_type: doc.return_type.try_clone()?,
        })
    }
}

#[cfg(feature = "doc")]
fn context_to_arguments(
    args: Option<usize>,
    types: &[meta::DocType],
    names: &[String],
) -> alloc::Result<Option<Box<[meta::DocArgument]>>> {
    use core::iter;

    let Some(args) = args else {
        return Ok(None);
    };

    let len = args.max(types.len()).max(names.len()).max(names.len());
    let mut out = Vec::try_with_capacity(len)?;

    let mut types = types.iter();

    let names = names
        .iter()
        .map(|name| Some(name.as_str()))
        .chain(iter::repeat(None));

    for (n, name) in (0..len).zip(names) {
        let empty;

        let ty = match types.next() {
            Some(ty) => ty,
            None => {
                empty = meta::DocType::empty();
                &empty
            }
        };

        out.try_push(meta::DocArgument {
            name: match name {
                Some(name) => meta::DocName::Name(Box::try_from(name)?),
                None => meta::DocName::Index(n),
            },
            base: ty.base,
            generics: ty.generics.try_clone()?,
        })?;
    }

    Ok(Some(Box::try_from(out)?))
}

/// A name inside of a document.
#[derive(Debug, TryClone)]
#[cfg(feature = "doc")]
pub(crate) enum DocName {
    /// A string name.
    Name(Box<str>),
    /// A numbered name.
    Index(#[try_clone(copy)] usize),
}

#[cfg(feature = "cli")]
impl DocName {
    pub(crate) fn is_self(&self) -> bool {
        match self {
            DocName::Name(name) => name.as_ref() == "self",
            DocName::Index(..) => false,
        }
    }
}

#[cfg(feature = "doc")]
impl fmt::Display for DocName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocName::Name(name) => write!(f, "{name}"),
            DocName::Index(index) if *index == 0 => write!(f, "value"),
            DocName::Index(index) => write!(f, "value{index}"),
        }
    }
}

/// A description of a type.
#[derive(Debug, TryClone)]
#[cfg(feature = "doc")]
pub(crate) struct DocArgument {
    /// The name of an argument.
    pub(crate) name: DocName,
    /// The base type.
    pub(crate) base: Hash,
    /// Generic parameters.
    pub(crate) generics: Box<[DocType]>,
}

/// A description of a type.
#[derive(Default, Debug, TryClone)]
pub struct DocType {
    /// The base type.
    #[cfg(feature = "doc")]
    pub(crate) base: Hash,
    /// Generic parameters.
    #[cfg(feature = "doc")]
    pub(crate) generics: Box<[DocType]>,
}

impl DocType {
    /// Construct an empty type documentation.
    pub(crate) fn empty() -> Self {
        Self::new(Hash::EMPTY)
    }

    /// Construct type documentation.
    #[cfg_attr(not(feature = "doc"), allow(unused_variables))]
    pub fn with_generics<const N: usize>(
        base: Hash,
        generics: [DocType; N],
    ) -> alloc::Result<Self> {
        Ok(Self {
            #[cfg(feature = "doc")]
            base,
            #[cfg(feature = "doc")]
            generics: Box::try_from(generics)?,
        })
    }

    /// Construct type with the specified base type.
    #[cfg_attr(not(feature = "doc"), allow(unused_variables))]
    pub(crate) fn new(base: Hash) -> Self {
        Self {
            #[cfg(feature = "doc")]
            base,
            #[cfg(feature = "doc")]
            generics: Box::default(),
        }
    }
}

/// The kind of an associated function.
#[derive(Debug, TryClone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AssociatedKind {
    /// A protocol function implemented on the type itself.
    Protocol(&'static Protocol),
    /// A field function with the given protocol.
    FieldFn(&'static Protocol, Cow<'static, str>),
    /// An index function with the given protocol.
    IndexFn(&'static Protocol, usize),
    /// The instance function refers to the given named instance fn.
    Instance(Cow<'static, str>),
}

impl AssociatedKind {
    /// Convert the kind into a hash function.
    pub(crate) fn hash(&self, instance_type: Hash) -> Hash {
        match self {
            Self::Protocol(protocol) => Hash::associated_function(instance_type, protocol.hash),
            Self::IndexFn(protocol, index) => {
                Hash::index_function(protocol.hash, instance_type, Hash::index(*index))
            }
            Self::FieldFn(protocol, field) => {
                Hash::field_function(protocol.hash, instance_type, field.as_ref())
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
