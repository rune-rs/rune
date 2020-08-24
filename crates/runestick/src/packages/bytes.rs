//! The bytes package, providing access to the bytes type.

use crate::bytes::Bytes;
use crate::context::{ContextError, Module};

/// Get the module for the bytes package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "bytes"]);

    module.ty(&["Bytes"]).build::<Bytes>()?;
    module.function(&["Bytes", "new"], Bytes::new)?;
    module.function(&["Bytes", "with_capacity"], Bytes::with_capacity)?;
    module.function(&["Bytes", "from_vec"], Bytes::from_vec)?;

    module.inst_fn("as_vec", Bytes::as_vec)?;
    module.inst_fn("extend", Bytes::extend)?;
    module.inst_fn("extend_str", Bytes::extend_str)?;
    module.inst_fn("pop", Bytes::pop)?;
    module.inst_fn("last", Bytes::last)?;

    module.inst_fn("len", Bytes::len)?;
    module.inst_fn("capacity", Bytes::capacity)?;
    module.inst_fn("clear", Bytes::clear)?;
    module.inst_fn("reserve", Bytes::reserve)?;
    module.inst_fn("reserve_exact", Bytes::reserve_exact)?;
    module.inst_fn("clone", Bytes::clone)?;
    module.inst_fn("shrink_to_fit", Bytes::shrink_to_fit)?;
    Ok(module)
}
