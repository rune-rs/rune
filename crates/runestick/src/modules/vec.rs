//! The `std::vec` module.

use crate::{ContextError, Module, Vec};

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "vec"]);

    module.ty::<Vec>()?;

    module.function(&["Vec", "new"], Vec::new)?;
    module.inst_fn("iter", vec_iter)?;
    module.inst_fn("len", Vec::len)?;
    module.inst_fn("push", Vec::push)?;
    module.inst_fn("clear", Vec::clear)?;
    module.inst_fn("pop", Vec::pop)?;

    module.inst_fn(crate::INTO_ITER, vec_iter)?;
    Ok(module)
}

fn vec_iter(vec: &Vec) -> crate::Iterator {
    crate::Iterator::from_double_ended("std::vec::Iter", vec.clone().into_iter())
}
