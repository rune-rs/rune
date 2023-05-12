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
    module.function_meta(Bytes::__extend__meta)?;
    module.function_meta(Bytes::__extend_str__meta)?;
    module.function_meta(Bytes::__pop__meta)?;
    module.function_meta(Bytes::__last__meta)?;

    module.associated_function("len", Bytes::len)?;
    module.associated_function("capacity", Bytes::capacity)?;
    module.associated_function("clear", Bytes::clear)?;
    module.associated_function("reserve", Bytes::reserve)?;
    module.associated_function("reserve_exact", Bytes::reserve_exact)?;
    module.associated_function("clone", Bytes::clone)?;
    module.associated_function("shrink_to_fit", Bytes::shrink_to_fit)?;
    Ok(module)
}
