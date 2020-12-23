mod index;
mod index_local;
mod index_scopes;

pub(crate) use self::index::{Index, Indexer};
pub(crate) use self::index_local::IndexLocal;
pub(crate) use self::index_scopes::{IndexFnKind, IndexScopes};
