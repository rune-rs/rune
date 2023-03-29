use thiserror::Error;

use crate::compile::meta;
use crate::compile::{ItemBuf, Meta};
use crate::runtime::{TypeInfo, VmError};
use crate::Hash;

/// An error raised when building the context.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum ContextError {
    #[error("`()` types are already present")]
    UnitAlreadyPresent,
    #[error("`{name}` types are already present")]
    InternalAlreadyPresent { name: &'static str },
    #[error("conflicting meta {existing} while trying to insert {current}")]
    ConflictingMeta {
        current: Box<Meta>,
        existing: Box<Meta>,
    },
    #[error("function `{signature}` ({hash}) already exists")]
    ConflictingFunction {
        signature: Box<meta::Signature>,
        hash: Hash,
    },
    #[error("function with name `{name}` already exists")]
    ConflictingFunctionName { name: ItemBuf },
    #[error("constant with name `{name}` already exists")]
    ConflictingConstantName { name: ItemBuf },
    #[error("instance function `{name}` for type `{type_info}` already exists")]
    ConflictingInstanceFunction { type_info: TypeInfo, name: Box<str> },
    #[error("protocol function `{name}` for type `{type_info}` already exists")]
    ConflictingProtocolFunction { type_info: TypeInfo, name: Box<str> },
    #[error("protocol function with hash `{hash}` for type `{type_info}` already exists")]
    ConflictingInstanceFunctionHash { type_info: TypeInfo, hash: Hash },
    #[error("module `{item}` with hash `{hash}` already exists")]
    ConflictingModule { item: ItemBuf, hash: Hash },
    #[error("type `{item}` already exists `{type_info}`")]
    ConflictingType { item: ItemBuf, type_info: TypeInfo },
    #[error("type `{item}` at `{type_info}` already has a specification")]
    ConflictingTypeMeta { item: ItemBuf, type_info: TypeInfo },
    #[error("type `{item}` with info `{type_info}` isn't registered")]
    MissingType { item: ItemBuf, type_info: TypeInfo },
    #[error("type `{item}` with info `{type_info}` is registered but is not an enum")]
    MissingEnum { item: ItemBuf, type_info: TypeInfo },
    #[error("tried to insert conflicting hash `{hash}` for `{existing}`")]
    ConflictingTypeHash { hash: Hash, existing: Hash },
    #[error("variant with `{item}` already exists")]
    ConflictingVariant { item: ItemBuf },
    #[error("instance `{instance_type}` does not exist in module")]
    MissingInstance { instance_type: TypeInfo },
    #[error("error when converting to constant value: {error}")]
    ValueError { error: VmError },
    #[error("missing variant {index} for `{type_info}`")]
    MissingVariant { type_info: TypeInfo, index: usize },
    #[error("constructor for variant {index} in `{type_info}` has already been registered")]
    VariantConstructorConflict { type_info: TypeInfo, index: usize },
}
