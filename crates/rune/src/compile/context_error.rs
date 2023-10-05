use core::fmt;

use crate::alloc::{self, Box};
use crate::compile::ItemBuf;
use crate::runtime::{TypeInfo, VmError};
use crate::Hash;

/// An error raised when building the context.
#[derive(Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum ContextError {
    AllocError {
        error: alloc::Error,
    },
    UnitAlreadyPresent,
    InternalAlreadyPresent {
        name: &'static str,
    },
    ConflictingFunction {
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
    ConflictingTypeMeta {
        item: ItemBuf,
        type_info: TypeInfo,
    },
    ConflictingVariantMeta {
        index: usize,
        type_info: TypeInfo,
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
    ValueError {
        error: VmError,
    },
    VariantConstructorConflict {
        type_info: TypeInfo,
        index: usize,
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
            ContextError::UnitAlreadyPresent {} => {
                write!(f, "Unit `()` type is already present")?;
            }
            ContextError::InternalAlreadyPresent { name } => {
                write!(f, "Type for name `{name}` is already present")?;
            }
            ContextError::ConflictingFunction { hash } => {
                write!(f, "Function with hash `{hash}` already exists")?;
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
            ContextError::ConflictingTypeMeta { item, type_info } => {
                write!(
                    f,
                    "Type `{item}` at `{type_info}` already has a specification"
                )?;
            }
            ContextError::ConflictingVariantMeta { index, type_info } => {
                write!(
                    f,
                    "Variant `{index}` for `{type_info}` already has a specification"
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
                write!(
                    f,
                    "Constructor for type `{type_info}` has already been registered"
                )?;
            }
            ContextError::ValueError { error } => {
                write!(f, "Error when converting to constant value: {error}")?;
            }
            ContextError::VariantConstructorConflict { type_info, index } => {
                write!(
                    f,
                    "Constructor for variant {index} in `{type_info}` has already been registered"
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
                write!(f,"Type hash mismatch for `{type_info}`, from module is `{hash}` while from item `{item}` is `{item_hash}`. A possibility is that it has the wrong #[rune(item = ..)] setting.")?;
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

#[cfg(feature = "std")]
impl std::error::Error for ContextError {}
