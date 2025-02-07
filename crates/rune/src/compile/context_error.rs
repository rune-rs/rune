use core::fmt;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::compile::meta::AssociatedKind;
use crate::runtime::{RuntimeError, TypeInfo};
use crate::{Hash, ItemBuf};

/// An error raised when building the context.
#[derive(Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum ContextError {
    AllocError {
        error: alloc::Error,
    },
    InvalidConstValue {
        item: ItemBuf,
        error: Box<RuntimeError>,
    },
    InvalidAssociatedConstValue {
        container: TypeInfo,
        kind: Box<AssociatedKind>,
        error: Box<RuntimeError>,
    },
    UnitAlreadyPresent,
    InternalAlreadyPresent {
        name: &'static str,
    },
    ConflictingFunction {
        part: Box<str>,
        hash: Hash,
    },
    ConflictingFunctionName {
        item: ItemBuf,
        hash: Hash,
    },
    ConflictingMacroName {
        item: ItemBuf,
        hash: Hash,
    },
    ConflictingConstantName {
        item: ItemBuf,
        hash: Hash,
    },
    ConflictingInstanceFunction {
        type_info: TypeInfo,
        name: Box<str>,
    },
    ConflictingProtocolFunction {
        type_info: TypeInfo,
        name: Box<str>,
    },
    ConflictingFieldFunction {
        type_info: TypeInfo,
        name: Box<str>,
        field: Box<str>,
    },
    ConflictingIndexFunction {
        type_info: TypeInfo,
        name: Box<str>,
        index: usize,
    },
    ConflictingModule {
        item: ItemBuf,
        hash: Hash,
    },
    ConflictingType {
        item: ItemBuf,
        type_info: TypeInfo,
        hash: Hash,
    },
    ConflictingReexport {
        item: ItemBuf,
        hash: Hash,
        to: ItemBuf,
    },
    ConflictingTrait {
        item: ItemBuf,
        hash: Hash,
    },
    ConflictingTraitImpl {
        trait_item: ItemBuf,
        trait_hash: Hash,
        item: ItemBuf,
        hash: Hash,
    },
    MissingTraitFunction {
        name: String,
        item: ItemBuf,
        hash: Hash,
        trait_item: ItemBuf,
        trait_hash: Hash,
    },
    MissingTrait {
        item: ItemBuf,
        hash: Hash,
        impl_item: ItemBuf,
        impl_hash: Hash,
    },
    ConflictingTypeMeta {
        item: ItemBuf,
        type_info: TypeInfo,
    },
    ConflictingVariantMeta {
        type_info: TypeInfo,
        name: &'static str,
    },
    ConflictingMetaHash {
        item: ItemBuf,
        hash: Hash,
        existing: Hash,
    },
    ConflictingTypeHash {
        hash: Hash,
        existing: Hash,
    },
    ConflictingVariant {
        item: ItemBuf,
    },
    ConstructorConflict {
        type_info: TypeInfo,
    },
    VariantConstructorConflict {
        type_info: TypeInfo,
        name: &'static str,
    },
    ConflictingConstConstruct {
        type_info: TypeInfo,
        hash: Hash,
    },
    MissingType {
        item: ItemBuf,
        type_info: TypeInfo,
    },
    MissingEnum {
        item: ItemBuf,
        type_info: TypeInfo,
    },
    MissingContainer {
        container: TypeInfo,
    },
    MissingVariant {
        index: usize,
        type_info: TypeInfo,
    },
    ExpectedAssociated,
    TypeHashMismatch {
        type_info: TypeInfo,
        item: ItemBuf,
        hash: Hash,
        item_hash: Hash,
    },
    StaticTypeHashMismatch {
        type_info: TypeInfo,
        item: ItemBuf,
        hash: Hash,
        item_hash: Hash,
    },
}

impl From<alloc::Error> for ContextError {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        ContextError::AllocError { error }
    }
}

impl From<alloc::alloc::AllocError> for ContextError {
    #[inline]
    fn from(error: alloc::alloc::AllocError) -> Self {
        ContextError::AllocError {
            error: error.into(),
        }
    }
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ContextError::AllocError { error } => {
                error.fmt(f)?;
            }
            ContextError::InvalidConstValue { item, error } => {
                write!(f, "Error when building constant {item}: {error}")?;
            }
            ContextError::InvalidAssociatedConstValue {
                container,
                kind,
                error,
            } => {
                write!(
                    f,
                    "Error when building associated constant in {container}::{kind}: {error}"
                )?;
            }
            ContextError::UnitAlreadyPresent {} => {
                write!(f, "Unit `()` type is already present")?;
            }
            ContextError::InternalAlreadyPresent { name } => {
                write!(f, "Type for name `{name}` is already present")?;
            }
            ContextError::ConflictingFunction { part, hash } => {
                write!(
                    f,
                    "Function with hash `{hash}` part of `{part}` already exists"
                )?;
            }
            ContextError::ConflictingFunctionName { item, hash } => {
                write!(f, "Function `{item}` already exists with hash `{hash}`")?;
            }
            ContextError::ConflictingMacroName { item, hash } => {
                write!(f, "Macro `{item}` already exists with hash `{hash}`")?;
            }
            ContextError::ConflictingConstantName { item, hash } => {
                write!(f, "Constant `{item}` already exists with hash `{hash}`")?;
            }
            ContextError::ConflictingInstanceFunction { type_info, name } => {
                write!(
                    f,
                    "Instance function `{name}` for type `{type_info}` already exists"
                )?;
            }
            ContextError::ConflictingProtocolFunction { type_info, name } => {
                write!(
                    f,
                    "Protocol function `{name}` for type `{type_info}` already exists"
                )?;
            }
            ContextError::ConflictingFieldFunction {
                type_info,
                name,
                field,
            } => {
                write!(f,"Field function `{name}` for field `{field}` and type `{type_info}` already exists")?;
            }
            ContextError::ConflictingIndexFunction {
                type_info,
                name,
                index,
            } => {
                write!(f,"Index function `{name}` for index `{index}` and type `{type_info}` already exists")?;
            }
            ContextError::ConflictingModule { item, hash } => {
                write!(f, "Module `{item}` with hash `{hash}` already exists")?;
            }
            ContextError::ConflictingType {
                item,
                type_info,
                hash,
            } => {
                write!(
                    f,
                    "Type `{item}` already exists `{type_info}` with hash `{hash}`"
                )?;
            }
            ContextError::ConflictingReexport { item, hash, to } => {
                write!(
                    f,
                    "Reexport at `{item}` with hash `{hash}` to `{to}` already exists"
                )?;
            }
            ContextError::ConflictingTrait { item, hash } => {
                write!(
                    f,
                    "Trait `{item}` with hash `{hash}` conflicts with other item in module"
                )?;
            }
            ContextError::ConflictingTraitImpl {
                trait_item,
                trait_hash,
                item,
                hash,
            } => {
                write!(
                    f,
                    "Trait `{trait_item}` with hash `{trait_hash}` is implemented multiple types for type `{item}` with hash `{hash}`"
                )?;
            }
            ContextError::MissingTraitFunction {
                name,
                item,
                hash,
                trait_item,
                trait_hash,
            } => {
                write!(
                    f,
                    "Missing required associated `{name}` for type `{item}` with hash `{hash}` when implementing trait `{trait_item}` with hash `{trait_hash}`"
                )?;
            }
            ContextError::MissingTrait {
                item,
                hash,
                impl_item,
                impl_hash,
            } => {
                write!(
                    f,
                    "Missing trait `{item}` with hash `{hash}` when implementing it for `{impl_item}` with hash `{impl_hash}`"
                )?;
            }
            ContextError::ConflictingTypeMeta { item, type_info } => {
                write!(
                    f,
                    "Type `{item}` at `{type_info}` already has a specification"
                )?;
            }
            ContextError::ConflictingVariantMeta { type_info, name } => {
                write!(
                    f,
                    "Variant `{name}` for `{type_info}` already has a specification"
                )?;
            }
            ContextError::ConflictingMetaHash {
                item,
                hash,
                existing,
            } => {
                write!(f,"Conflicting meta hash `{hash}` for existing `{existing}` when inserting item `{item}`")?;
            }
            ContextError::ConflictingTypeHash { hash, existing } => {
                write!(
                    f,
                    "Tried to insert conflicting hash `{hash}` for `{existing}`"
                )?;
            }
            ContextError::ConflictingVariant { item } => {
                write!(f, "Variant with `{item}` already exists")?;
            }
            ContextError::ConstructorConflict { type_info } => {
                write!(f, "Constructor for `{type_info}` already exists")?;
            }
            ContextError::VariantConstructorConflict { type_info, name } => {
                write!(
                    f,
                    "Constructor for variant `{name}` for `{type_info}` already exists"
                )?;
            }
            ContextError::ConflictingConstConstruct { type_info, hash } => {
                write!(
                    f,
                    "Constant constructor with hash {hash} for `{type_info}` already exists"
                )?;
            }
            ContextError::MissingType { item, type_info } => {
                write!(f, "Type `{item}` with info `{type_info}` isn't registered")?;
            }
            ContextError::MissingEnum { item, type_info } => {
                write!(
                    f,
                    "Type `{item}` with info `{type_info}` is registered but is not an enum"
                )?;
            }
            ContextError::MissingContainer { container } => {
                write!(f, "Container `{container}` is not registered")?;
            }
            ContextError::MissingVariant { index, type_info } => {
                write!(f, "Missing variant {index} for `{type_info}`")?;
            }
            ContextError::ExpectedAssociated {} => {
                write!(f, "Expected associated function")?;
            }
            ContextError::TypeHashMismatch {
                type_info,
                item,
                hash,
                item_hash,
            } => {
                let expected = item.parent().unwrap_or_default();

                write! {
                    f,
                    "Type hash mismatch for `{type_info}`, from module is `{hash}` while from item `{item}` is `{item_hash}`.\n\
                    You might not have the #[rune(item = {expected})] attribute set."
                }?;
            }
            ContextError::StaticTypeHashMismatch {
                type_info,
                item,
                hash,
                item_hash,
            } => {
                write!(f, "Static type hash mismatch for `{type_info}`, from module is `{hash}` while from item `{item}` is `{item_hash}`. The static item might be registered in the wrong module, or that the static type hash is miscalculated.")?;
            }
        }

        Ok(())
    }
}

impl core::error::Error for ContextError {}
