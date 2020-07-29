//! Package containing string functions.

use crate::functions::{Module, RegisterError};
use crate::packages::bytes::Bytes;

/// into_bytes shim for strings.
fn into_bytes(s: String) -> Bytes {
    Bytes::from_bytes(s.into_bytes())
}

/// Get the module for the string package.
pub fn module() -> Result<Module, RegisterError> {
    let mut module = Module::new(&["string"]);
    module.global_fn("new", String::new)?;
    module.global_fn("with_capacity", String::with_capacity)?;

    module.instance_fn("len", String::len)?;
    module.instance_fn("capacity", String::capacity)?;
    module.instance_fn("clear", String::clear)?;
    module.instance_fn("push_str", String::push_str)?;
    module.instance_fn("reserve", String::reserve)?;
    module.instance_fn("reserve_exact", String::reserve_exact)?;
    module.instance_fn("into_bytes", into_bytes)?;
    module.instance_fn("clone", String::clone)?;
    module.instance_fn("shrink_to_fit", String::shrink_to_fit)?;
    Ok(module)
}
