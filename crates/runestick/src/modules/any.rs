//! `std::any` module.

use crate as runestick; // for #[derive(Any)] to work
use crate::{Any, ContextError, Interface, Module, Protocol, Value};
use std::any::TypeId as StdTypeId;
use std::fmt;
use std::fmt::Write as _;

#[derive(Any, Debug)]
#[repr(transparent)]
struct TypeId(StdTypeId);

fn type_name_of_val(iface: Interface) -> String {
    // This should never fail
    iface
        .into_type_name()
        .unwrap_or_else(|_| String::from("<unknown type>"))
}

fn type_id_of_val(item: Value) -> TypeId {
    unsafe { std::mem::transmute(item.type_hash().expect("no type known for item!")) }
}

fn format_type_id(item: &TypeId, buf: &mut String) -> fmt::Result {
    write!(buf, "{:?}", item.0)
}

/// Construct the `std::any` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["any"]);

    module.function(&["type_name_of_val"], type_name_of_val)?;

    module.ty::<TypeId>()?;
    module.function(&["TypeId", "of_val"], type_id_of_val)?;
    module.inst_fn(Protocol::STRING_DISPLAY, format_type_id)?;
    Ok(module)
}
