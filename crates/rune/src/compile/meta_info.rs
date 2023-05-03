use core::fmt;

use crate::no_std::prelude::*;

use crate::compile::meta;
use crate::compile::{Item, ItemBuf};

/// Provides an owned human-readable description of a meta item.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MetaInfo {
    /// The kind of the item.
    kind: MetaInfoKind,
    /// The item being described.
    item: ItemBuf,
}

impl MetaInfo {
    /// Construct a new meta info.
    pub(crate) fn new(kind: &meta::Kind, item: &Item) -> Self {
        Self {
            kind: MetaInfoKind::from_kind(kind),
            item: item.to_owned(),
        }
    }
}

impl fmt::Display for MetaInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            MetaInfoKind::Unknown => {
                write!(fmt, "unknown {}", self.item)?;
            }
            MetaInfoKind::Struct => {
                write!(fmt, "struct {}", self.item)?;
            }
            MetaInfoKind::Variant => {
                write!(fmt, "variant {}", self.item)?;
            }
            MetaInfoKind::Enum => {
                write!(fmt, "enum {}", self.item)?;
            }
            MetaInfoKind::Function => {
                write!(fmt, "fn {}", self.item)?;
            }
            MetaInfoKind::Closure => {
                write!(fmt, "closure {}", self.item)?;
            }
            MetaInfoKind::AsyncBlock => {
                write!(fmt, "async block {}", self.item)?;
            }
            MetaInfoKind::Const => {
                write!(fmt, "const {}", self.item)?;
            }
            MetaInfoKind::ConstFn => {
                write!(fmt, "const fn {}", self.item)?;
            }
            MetaInfoKind::Import => {
                write!(fmt, "import {}", self.item)?;
            }
            MetaInfoKind::Module => {
                write!(fmt, "module {}", self.item)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum MetaInfoKind {
    Unknown,
    Struct,
    Variant,
    Enum,
    Function,
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
            meta::Kind::Unknown { .. } => MetaInfoKind::Unknown,
            meta::Kind::Struct { .. } => MetaInfoKind::Struct,
            meta::Kind::Variant { .. } => MetaInfoKind::Variant,
            meta::Kind::Enum { .. } => MetaInfoKind::Enum,
            meta::Kind::Function { .. } => MetaInfoKind::Function,
            meta::Kind::Closure { .. } => MetaInfoKind::Closure,
            meta::Kind::AsyncBlock { .. } => MetaInfoKind::AsyncBlock,
            meta::Kind::Const { .. } => MetaInfoKind::Const,
            meta::Kind::ConstFn { .. } => MetaInfoKind::ConstFn,
            meta::Kind::Import { .. } => MetaInfoKind::Import,
            meta::Kind::Module { .. } => MetaInfoKind::Module,
        }
    }
}
