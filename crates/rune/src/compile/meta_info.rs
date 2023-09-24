use core::fmt;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::compile::{meta, Item, ItemBuf};
use crate::Hash;

/// Provides an owned human-readable description of a meta item.
#[derive(Debug)]
#[non_exhaustive]
pub struct MetaInfo {
    /// The kind of the item.
    kind: MetaInfoKind,
    /// The hash of the meta item.
    hash: Hash,
    /// The item being described.
    item: Option<ItemBuf>,
}

impl MetaInfo {
    /// Construct a new meta info.
    pub(crate) fn new(kind: &meta::Kind, hash: Hash, item: Option<&Item>) -> alloc::Result<Self> {
        Ok(Self {
            kind: MetaInfoKind::from_kind(kind),
            hash,
            item: item.map(|item| item.try_to_owned()).transpose()?,
        })
    }
}

impl fmt::Display for MetaInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Name<'a>(Hash, Option<&'a Item>);

        impl fmt::Display for Name<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if let Some(item) = self.1 {
                    item.fmt(f)
                } else {
                    self.0.fmt(f)
                }
            }
        }

        let name = Name(self.hash, self.item.as_deref());

        match self.kind {
            MetaInfoKind::Type => {
                write!(fmt, "type {name}")?;
            }
            MetaInfoKind::Struct => {
                write!(fmt, "struct {name}")?;
            }
            MetaInfoKind::Variant => {
                write!(fmt, "variant {name}")?;
            }
            MetaInfoKind::Enum => {
                write!(fmt, "enum {name}")?;
            }
            MetaInfoKind::Macro => {
                write!(fmt, "macro {name}")?;
            }
            MetaInfoKind::AttributeMacro => {
                write!(fmt, "attribute macro {name}")?;
            }
            MetaInfoKind::Function => {
                write!(fmt, "fn {name}")?;
            }
            MetaInfoKind::Associated => {
                write!(fmt, "associated fn {name}")?;
            }
            MetaInfoKind::Closure => {
                write!(fmt, "closure {name}")?;
            }
            MetaInfoKind::AsyncBlock => {
                write!(fmt, "async block {name}")?;
            }
            MetaInfoKind::Const => {
                write!(fmt, "const {name}")?;
            }
            MetaInfoKind::ConstFn => {
                write!(fmt, "const fn {name}")?;
            }
            MetaInfoKind::Import => {
                write!(fmt, "import {name}")?;
            }
            MetaInfoKind::Module => {
                write!(fmt, "module {name}")?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum MetaInfoKind {
    Type,
    Struct,
    Variant,
    Enum,
    Macro,
    AttributeMacro,
    Function,
    Associated,
    Closure,
    AsyncBlock,
    Const,
    ConstFn,
    Import,
    Module,
}

impl MetaInfoKind {
    fn from_kind(value: &meta::Kind) -> Self {
        match value {
            meta::Kind::Type { .. } => MetaInfoKind::Type,
            meta::Kind::Struct { .. } => MetaInfoKind::Struct,
            meta::Kind::Variant { .. } => MetaInfoKind::Variant,
            meta::Kind::Enum { .. } => MetaInfoKind::Enum,
            meta::Kind::Macro { .. } => MetaInfoKind::Macro,
            meta::Kind::AttributeMacro { .. } => MetaInfoKind::AttributeMacro,
            meta::Kind::Function {
                associated: None, ..
            } => MetaInfoKind::Function,
            meta::Kind::Function {
                associated: Some(..),
                ..
            } => MetaInfoKind::Associated,
            meta::Kind::Closure { .. } => MetaInfoKind::Closure,
            meta::Kind::AsyncBlock { .. } => MetaInfoKind::AsyncBlock,
            meta::Kind::Const { .. } => MetaInfoKind::Const,
            meta::Kind::ConstFn { .. } => MetaInfoKind::ConstFn,
            meta::Kind::Import { .. } => MetaInfoKind::Import,
            meta::Kind::Module { .. } => MetaInfoKind::Module,
        }
    }
}
