pub(crate) mod index;
mod index_scopes;
mod locals;

pub(crate) use self::index::Indexer;
pub(crate) use self::index_scopes::{IndexFnKind, IndexScopes};
