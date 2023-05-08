//! `std::bytes` module.

use crate::runtime::Bytes;
use crate::{ContextError, Module};

/// Construct the `std::bytes` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["bytes"]);

    module.ty::<Bytes>()?;
    module.function_meta(Bytes::__new__meta)?;
    module.function(["Bytes", "with_capacity"], Bytes::with_capacity)?;
    module.function_meta(Bytes::__from_vec__meta)?;

    module.function_meta(Bytes::__into_vec__meta)?;
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
