mod arena;
pub(crate) use self::arena::Arena;

mod hir;
pub(crate) use self::hir::*;

pub(crate) mod lowering;

pub(crate) mod scopes;
pub(crate) use self::scopes::{Scope, Scopes, Variable};
