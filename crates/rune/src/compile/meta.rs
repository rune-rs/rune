//! Compiler metadata for Rune.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use crate::ast::{LitStr, Span};
use crate::collections::HashSet;
use crate::compile::attrs::Attributes;
use crate::compile::{
    InstFnKind, Item, ItemBuf, ItemId, Location, MetaInfo, ModId, Pool, Visibility,
};
use crate::hash::Hash;
use crate::parse::{Id, ParseError, ResolveContext};
use crate::runtime::{ConstValue, TypeInfo};

/// Provides a human-readable description of a meta item. This is cheaper to use
/// than [Meta] because it avoids having to clone some data.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MetaRef<'a> {
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
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SourceMeta {
    /// The location of the compile source.
    pub location: Location,
    /// The optional path where the meta is declared.
    pub path: Option<Box<Path>>,
}

/// Doc content for a compiled item.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Doc {
    /// The span of the whole doc comment.
    pub(crate) span: Span,
    /// The string content of the doc comment.
    pub(crate) doc_string: LitStr,
}

impl Doc {
    pub(crate) fn collect_from(
        ctx: ResolveContext<'_>,
        attrs: &mut Attributes,
    ) -> Result<Vec<Doc>, ParseError> {
        Ok(attrs
            .try_parse_collect::<crate::compile::attrs::Doc>(ctx)?
            .into_iter()
            .map(|(span, doc)| Doc {
                span,
                doc_string: doc.doc_string,
            })
            .collect())
    }
}

/// Metadata about a compiled unit.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) struct Meta {
    /// Hash of the private metadata.
    pub(crate) hash: Hash,
    /// The item of the returned compile meta.
    pub(crate) item_meta: ItemMeta,
    /// The kind of the compile meta.
    pub(crate) kind: Kind,
    /// The source of the meta.
    pub(crate) source: Option<SourceMeta>,
}

impl Meta {
    /// Get the [Meta] which describes metadata.
    pub(crate) fn info(&self, pool: &Pool) -> MetaInfo {
        MetaInfo::new(&self.kind, pool.item(self.item_meta.item))
    }

    /// Get the [MetaRef] which describes this [meta::Meta] object.
    pub(crate) fn as_meta_ref<'a>(&'a self, pool: &'a Pool) -> MetaRef<'a> {
        MetaRef {
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
            Kind::Unknown { .. } => Some(self.hash),
            Kind::Struct { .. } => Some(self.hash),
            Kind::Enum { .. } => Some(self.hash),
            Kind::Function { .. } => Some(self.hash),
            Kind::Closure { .. } => Some(self.hash),
            Kind::AsyncBlock { .. } => Some(self.hash),
            Kind::Variant { .. } => None,
            Kind::Const { .. } => None,
            Kind::ConstFn { .. } => None,
            Kind::Import { .. } => None,
            Kind::Module => None,
        }
    }
}

/// The kind of a variant.
#[derive(Debug, Clone)]
pub enum Variant {
    /// A tuple variant.
    Tuple(Tuple),
    /// A struct variant.
    Struct(Struct),
    /// A unit variant.
    Unit,
}

/// Compile-time metadata kind about a unit.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Kind {
    /// The type is completely opaque. We have no idea about what it is with the
    /// exception of it having a type hash.
    Unknown,
    /// Metadata about a struct.
    Struct {
        /// Variant metadata.
        variant: Variant,
    },
    /// Metadata about an empty variant.
    Variant {
        /// Type hash of the enum this unit variant belongs to.
        enum_hash: Hash,
        /// The index of the variant.
        index: usize,
        /// Variant metadata.
        variant: Variant,
    },
    /// An enum item.
    Enum,
    /// A function declaration.
    Function {
        /// The number of arguments the function takes.
        args: Option<usize>,
        /// Whether this function has a `#[test]` annotation
        is_test: bool,
        /// Whether this function has a `#[bench]` annotation.
        is_bench: bool,
        /// Indicates that the function is an instance function.
        instance_function: bool,
    },
    /// A closure.
    Closure {
        /// Sequence of captured variables.
        captures: Arc<[String]>,
        /// If the closure moves its environment.
        do_move: bool,
    },
    /// An async block.
    AsyncBlock {
        /// Sequence of captured variables.
        captures: Arc<[String]>,
        /// If the async block moves its environment.
        do_move: bool,
    },
    /// The constant expression.
    Const {
        /// The evaluated constant value.
        const_value: ConstValue,
    },
    /// A constant function.
    ConstFn {
        /// Opaque identifier for the constant function.
        id: Id,
    },
    /// Purely an import.
    Import(Import),
    /// A module.
    Module,
}

/// An imported entry.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Import {
    /// The location of the import.
    pub(crate) location: Location,
    /// The item being imported.
    pub(crate) target: ItemId,
    /// The module in which the imports is located.
    pub(crate) module: ModId,
}

/// The metadata about a struct.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Struct {
    /// Fields associated with the type.
    pub(crate) fields: HashSet<Box<str>>,
}

/// The metadata about a tuple.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Tuple {
    /// The number of arguments the variant takes.
    pub(crate) args: usize,
    /// Hash of the constructor function.
    pub(crate) hash: Hash,
}

/// Item and the module that the item belongs to.
#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ItemMeta {
    /// The id of the item.
    pub(crate) id: Id,
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
#[derive(Debug, Clone)]
pub enum Signature {
    /// An unbound or static function
    Function {
        /// The type hash of the function
        type_hash: Hash,
        /// Path to the function.
        item: ItemBuf,
        /// Arguments.
        args: Option<usize>,
    },
    /// An instance function or method
    Instance {
        /// The type hash of the function
        type_hash: Hash,
        /// Path to the instance function.
        item: ItemBuf,
        /// Name of the instance function.
        name: InstFnKind,
        /// Arguments.
        args: Option<usize>,
        /// Information on the self type.
        self_type_info: TypeInfo,
    },
}

impl fmt::Display for Signature {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Function { item, args, .. } => {
                write!(fmt, "{}(", item)?;

                if let Some(args) = args {
                    let mut it = 0..*args;
                    let last = it.next_back();

                    for n in it {
                        write!(fmt, "#{}, ", n)?;
                    }

                    if let Some(n) = last {
                        write!(fmt, "#{}", n)?;
                    }
                } else {
                    write!(fmt, "...")?;
                }

                write!(fmt, ")")?;
            }
            Self::Instance {
                item,
                name,
                self_type_info,
                args,
                ..
            } => {
                write!(fmt, "{}::{}(self: {}", item, name, self_type_info)?;

                if let Some(args) = args {
                    for n in 0..*args {
                        write!(fmt, ", #{}", n)?;
                    }
                } else {
                    write!(fmt, ", ...")?;
                }

                write!(fmt, ")")?;
            }
        }

        Ok(())
    }
}
