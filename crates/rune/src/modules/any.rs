//! `std::any` module.

use crate::runtime::{Protocol, Value, VmResult};
use crate::{Any, ContextError, Module};
use std::any::TypeId as StdTypeId;
use std::fmt;
use std::fmt::Write;

#[derive(Any, Debug)]
#[rune(module = "crate")]
#[repr(transparent)]
struct TypeId(StdTypeId);

fn type_id_of_val(item: Value) -> VmResult<TypeId> {
    VmResult::Ok(unsafe { std::mem::transmute(vm_try!(item.type_hash())) })
}

fn format_type_id(item: &TypeId, buf: &mut String) -> fmt::Result {
    write!(buf, "{:?}", item.0)
}

/// Construct the `std::any` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["any"]);

    module.function(["type_name_of_val"], Value::into_type_name)?;

    module.ty::<TypeId>()?;
    module.function(["TypeId", "of_val"], type_id_of_val)?;
    module.inst_fn(Protocol::STRING_DISPLAY, format_type_id)?;
    Ok(module)
}
