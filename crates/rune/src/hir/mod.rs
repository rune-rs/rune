#[macro_use]
mod macros;

mod arena;
pub(crate) use self::arena::Arena;

mod hir;
pub(crate) use self::hir::*;

pub(crate) mod lowering;
pub(crate) mod lowering2;

pub(crate) mod scopes;
pub(crate) use self::scopes::Scopes;

pub(crate) mod interpreter;

mod ctxt;
pub(crate) use self::ctxt::Ctxt;
use self::ctxt::Needs;

pub(crate) mod typeck;
pub(crate) use self::typeck::{check_struct_literal_if_typed_with_item, TypeCheckState};
