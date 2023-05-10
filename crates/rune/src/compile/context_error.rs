use crate::no_std as std;
use crate::no_std::prelude::*;
use crate::no_std::thiserror;

use thiserror::Error;

use crate::compile::ItemBuf;
use crate::runtime::{TypeInfo, VmError};
use crate::Hash;

/// An error raised when building the context.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum ContextError {
    #[error("Unit `()` type is already present")]
    UnitAlreadyPresent,
    #[error("Type for name `{name}` is already present")]
    InternalAlreadyPresent { name: &'static str },
    #[error("Function with hash `{hash}` already exists")]
    ConflictingFunction { hash: Hash },
    #[error("Function `{item}` already exists with hash `{hash}`")]
    ConflictingFunctionName { item: ItemBuf, hash: Hash },
    #[error("Macro `{item}` already exists with hash `{hash}`")]
    ConflictingMacroName { item: ItemBuf, hash: Hash },
    #[error("Constant `{item}` already exists with hash `{hash}`")]
    ConflictingConstantName { item: ItemBuf, hash: Hash },
    #[error("Instance function `{name}` for type `{type_info}` already exists")]
    ConflictingInstanceFunction { type_info: TypeInfo, name: Box<str> },
    #[error("Protocol function `{name}` for type `{type_info}` already exists")]
    ConflictingProtocolFunction { type_info: TypeInfo, name: Box<str> },
    #[error("Field function `{name}` for field `{field}` and type `{type_info}` already exists")]
    ConflictingFieldFunction {
        type_info: TypeInfo,
        name: Box<str>,
        field: Box<str>,
    },
    #[error("Index function `{name}` for index `{index}` and type `{type_info}` already exists")]
    ConflictingIndexFunction {
        type_info: TypeInfo,
        name: Box<str>,
        index: usize,
    },
    #[error("Module `{item}` with hash `{hash}` already exists")]
    ConflictingModule { item: ItemBuf, hash: Hash },
    #[error("Type `{item}` already exists `{type_info}` with hash `{hash}`")]
    ConflictingType {
        item: ItemBuf,
        type_info: TypeInfo,
        hash: Hash,
    },
    #[error("Type `{item}` at `{type_info}` already has a specification")]
    ConflictingTypeMeta { item: ItemBuf, type_info: TypeInfo },
    #[error("Variant `{index}` for `{type_info}` already has a specification")]
    ConflictingVariantMeta { index: usize, type_info: TypeInfo },
    #[error(
        "Conflicting meta hash `{hash}` for existing `{existing}` when inserting item `{item}`"
    )]
    ConflictingMetaHash {
        item: ItemBuf,
        hash: Hash,
        existing: Hash,
    },
    #[error("Tried to insert conflicting hash `{hash}` for `{existing}`")]
    ConflictingTypeHash { hash: Hash, existing: Hash },
    #[error("Variant with `{item}` already exists")]
    ConflictingVariant { item: ItemBuf },
    #[error("Error when converting to constant value: {error}")]
    ValueError { error: VmError },
    #[error("Constructor for variant {index} in `{type_info}` has already been registered")]
    VariantConstructorConflict { type_info: TypeInfo, index: usize },
    #[error("Type `{item}` with info `{type_info}` isn't registered")]
    MissingType { item: ItemBuf, type_info: TypeInfo },
    #[error("Type `{item}` with info `{type_info}` is registered but is not an enum")]
    MissingEnum { item: ItemBuf, type_info: TypeInfo },
    #[error("Container `{container}` is not registered")]
    MissingContainer { container: TypeInfo },
    #[error("Missing variant {index} for `{type_info}`")]
    MissingVariant { index: usize, type_info: TypeInfo },
    #[error("Expected associated function")]
    ExpectedAssociated,
    #[error("Type hash mismatch for `{type_info}`, from module is `{hash}` while from item `{item}` is `{item_hash}`. A possibility is that it has the wrong #[rune(item = ..)] setting.")]
    TypeHashMismatch {
        type_info: TypeInfo,
        item: ItemBuf,
        hash: Hash,
        item_hash: Hash,
    },
}
