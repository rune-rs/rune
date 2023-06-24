//! `std::any` module.

use core::fmt::{self, Write};

use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::{Protocol, Type, Value, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::any` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["any"]);
    module
        .ty::<Type>()?
        .docs(["Represents a type in the Rune type system."]);
    module.function_meta(type_of_val)?;
    module.function_meta(type_name_of_val)?;
    module.associated_function(Protocol::STRING_DISPLAY, format_type)?;
    Ok(module)
}

/// Convert a value into a [`Type`] object.
///
/// # Examples
///
/// ```rune
/// let value1 = 42;
/// let value2 = 43;
/// let ty1 = Type::of_val(value1);
/// let ty2 = Type::of_val(value2);
/// assert_eq!(ty1, ty2);
/// ```
#[rune::function(path = Type::of_val)]
#[inline]
fn type_of_val(value: Value) -> VmResult<Type> {
    VmResult::Ok(Type::new(vm_try!(value.type_hash())))
}

fn format_type(ty: Type, buf: &mut String) -> fmt::Result {
    write!(buf, "{:?}", ty)
}

/// Get the type name of a value.
///
/// # Examples
///
/// ```rune
/// use std::any;
///
/// let value = 42;
/// assert_eq!(any::type_name_of_val(value), "::std::i64");
///
/// let value = [];
/// assert_eq!(any::type_name_of_val(value), "::std::vec::Vec");
/// ```
#[rune::function]
#[inline]
pub fn type_name_of_val(value: Value) -> VmResult<String> {
    value.into_type_name()
}
