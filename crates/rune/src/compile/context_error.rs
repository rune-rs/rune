use crate::no_std as std;
use crate::no_std::prelude::*;
use crate::no_std::thiserror;

use thiserror::Error;

use crate::compile::meta;
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
    #[error("Function `{signature}` ({hash}) already exists")]
    ConflictingFunction {
        signature: Box<meta::Signature>,
        hash: Hash,
    },
    #[error("Function `{item}` already exists")]
    ConflictingFunctionName { item: ItemBuf },
    #[error("Macro `{item}` already exists")]
    ConflictingMacroName { item: ItemBuf },
    #[error("Constant `{item}` already exists")]
    ConflictingConstantName { item: ItemBuf },
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
    #[error("Type `{item}` already exists `{type_info}`")]
    ConflictingType { item: ItemBuf, type_info: TypeInfo },
    #[error("Type `{item}` at `{type_info}` already has a specification")]
    ConflictingTypeMeta { item: ItemBuf, type_info: TypeInfo },
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
    #[error("Instance `{instance_type}` does not exist in module")]
    MissingInstance { instance_type: TypeInfo },
    #[error("Missing variant {index} for `{type_info}`")]
    MissingVariant { type_info: TypeInfo, index: usize },
    #[error("Expected associated function")]
    ExpectedAssociated,
}
