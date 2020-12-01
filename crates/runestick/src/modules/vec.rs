//! The `std::vec` module.

use crate::{ContextError, Module, Protocol, Vec};

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["vec"]);

    module.ty::<Vec>()?;

    module.function(&["Vec", "new"], Vec::new)?;
    module.inst_fn("extend", Vec::extend)?;
    module.inst_fn("iter", Vec::into_iterator)?;
    module.inst_fn("len", Vec::len)?;
    module.inst_fn("push", Vec::push)?;
    module.inst_fn("clear", Vec::clear)?;
    module.inst_fn("pop", Vec::pop)?;

    module.inst_fn(Protocol::INTO_ITER, Vec::into_iterator)?;

    // TODO: parameterize with generics.
    module.inst_fn("sort_int", sort_int)?;
    Ok(module)
}

/// Sort a vector of integers.
fn sort_int(vec: &mut Vec) {
    use crate::Value;

    vec.sort_by(|a, b| match (a, b) {
        (Value::Integer(a), Value::Integer(b)) => a.cmp(&b),
        // NB: fall back to sorting by address.
        _ => (a as *const _ as usize).cmp(&(b as *const _ as usize)),
    });
}
