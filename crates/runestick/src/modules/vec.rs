//! The `std::vec` module.

use crate::{ContextError, Module, Protocol, Vec};

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "vec"]);

    module.ty::<Vec>()?;

    module.function(&["Vec", "new"], Vec::new)?;
    module.inst_fn("extend", Vec::extend)?;
    module.inst_fn("iter", Vec::into_iterator)?;
    module.inst_fn("len", Vec::len)?;
    module.inst_fn("push", Vec::push)?;
    module.inst_fn("clear", Vec::clear)?;
    module.inst_fn("pop", Vec::pop)?;

    module.inst_fn(Protocol::INTO_ITER, Vec::into_iterator)?;
    Ok(module)
}
