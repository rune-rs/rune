//! `std::any` module.

use core::any;
use core::fmt::{self, Write};
use core::mem::transmute;

use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::{Protocol, Value, VmResult};
use crate::{Any, ContextError, Module};

#[derive(Any, Debug)]
#[rune(module = "crate")]
#[repr(transparent)]
struct TypeId(any::TypeId);

fn type_id_of_val(item: Value) -> VmResult<TypeId> {
    VmResult::Ok(unsafe { transmute(vm_try!(item.type_hash())) })
}

fn format_type_id(item: &TypeId, buf: &mut String) -> fmt::Result {
    write!(buf, "{:?}", item.0)
}

/// Construct the `std::any` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["any"]);

    module.function_meta(type_name_of_val)?;

    module.ty::<TypeId>()?;
    module.function(["TypeId", "of_val"], type_id_of_val)?;
    module.inst_fn(Protocol::STRING_DISPLAY, format_type_id)?;
    Ok(module)
}

/// Get the type name of a value.
///
/// # Examples
///
/// ```rune
/// use std::any;
///
/// let value = 42;
/// assert_eq!(any::type_name_of_val(value), "::std::int");
///
/// let value = [];
/// assert_eq!(any::type_name_of_val(value), "::std::vec::Vec");
/// ```
#[rune::function]
#[inline]
pub fn type_name_of_val(value: Value) -> VmResult<String> {
    value.into_type_name()
}
