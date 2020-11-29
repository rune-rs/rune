use crate::context::ContextSignature;
use crate::{CompileMeta, Hash, Item, TypeInfo};

use thiserror::Error;

/// An error raised when building the context.
#[derive(Debug, Error)]
pub enum ContextError {
    /// Conflicting `()` types.
    #[error("`()` types are already present")]
    UnitAlreadyPresent,
    /// Conflicting internal type.
    #[error("`{name}` types are already present")]
    InternalAlreadyPresent {
        /// The name of the internal type already present.
        name: &'static str,
    },
    /// A conflicting name.
    #[error("conflicting meta {existing} while trying to insert {current}")]
    ConflictingMeta {
        /// The current meta we tried to insert.
        current: Box<CompileMeta>,
        /// The existing meta item.
        existing: Box<CompileMeta>,
    },
    /// Error raised when attempting to register a conflicting function.
    #[error("function `{signature}` ({hash}) already exists")]
    ConflictingFunction {
        /// The signature of the conflicting function.
        signature: ContextSignature,
        /// The hash of the conflicting function.
        hash: Hash,
    },
    /// Error raised when attempting to register a conflicting function.
    #[error("function with name `{name}` already exists")]
    ConflictingFunctionName {
        /// The name of the conflicting function.
        name: Item,
    },
    /// Error raised when attempting to register a conflicting instance function.
    #[error("instance function `{name}` for type `{type_info}` already exists")]
    ConflictingInstanceFunction {
        /// Type that we register the instance function for.
        type_info: TypeInfo,
        /// The name of the conflicting function.
        name: String,
    },
    /// Tried to insert a module that conflicted with an already existing one.
    #[error("module `{item}` with hash `{hash}` already exists")]
    ConflictingModule {
        /// The name of the module that conflicted.
        item: Item,
        /// The hash of the module that conflicted.
        hash: Hash,
    },
    /// Raised when we try to register a conflicting type.
    #[error("type `{item}` already exists `{existing}`")]
    ConflictingType {
        /// The name we tried to register.
        item: Item,
        /// The type information for the type that already existed.
        existing: TypeInfo,
    },
    /// Raised when we try to register a conflicting type hash.
    #[error("tried to insert conflicting hash `{hash}` for `{existing}`")]
    ConflictingTypeHash {
        /// The hash we are trying to insert.
        hash: Hash,
        /// The hash that already existed.
        existing: Hash,
    },
    /// Error raised when attempting to register a conflicting function.
    #[error("variant with `{item}` already exists")]
    ConflictingVariant {
        /// The name of the conflicting variant.
        item: Item,
    },
    /// Error raised when attempting to register an instance function on an
    /// instance which does not exist.
    #[error("instance `{instance_type}` does not exist in module")]
    MissingInstance {
        /// The instance type.
        instance_type: TypeInfo,
    },
}
